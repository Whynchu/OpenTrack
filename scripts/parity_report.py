"""Validate the parity ledger and render evidence-based metrics (standard library only)."""
import argparse
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
STATES = {"pending", "pass", "fail", "stale"}


def validate(data):
    def require(ok, message):
        if not ok:
            raise ValueError(message)

    require(data["schema_version"] == 1, "Unsupported schema version")
    scope = data["scope"]
    require(bool(scope["version"]) and bool(scope["description"]), "Scope must be named")
    require(type(scope["baseline_complete"]) is bool, "baseline_complete must be boolean")
    evidence = data["evidence"]
    for key, item in evidence.items():
        require(item["kind"] in {"source", "runtime", "reference", "user", "manual", "inference"}, f"{key}: invalid evidence kind")
        require(bool(item["location"]), f"{key}: missing location")
        if item["kind"] in {"runtime", "source"}:
            require(bool(item.get("identity")), f"{key}: missing build/source identity")

    def refs(ids, kind, label, mandatory=False):
        require(isinstance(ids, list), f"{label}: evidence must be a list")
        require(not mandatory or bool(ids), f"{label}: missing {kind} evidence")
        for key in ids:
            require(key in evidence, f"{label}: unknown evidence {key}")
            require(evidence[key]["kind"] == kind, f"{label}: {key} is not {kind} evidence")

    def outcome(obj, label):
        require(obj["status"] in STATES, f"{label}: invalid result")
        refs(obj.get("evidence", []), "runtime", label, obj["status"] == "pass")

    requirements = data["requirements"]
    require(bool(requirements), "Empty requirements")
    ids = [r["id"] for r in requirements]
    require(len(ids) == len(set(ids)), "Duplicate requirement ID")
    for r in requirements:
        name = r["id"]
        require(type(r["weight"]) is int and r["weight"] in {1, 2, 3}, f"{name}: invalid weight")
        require(bool(r["outcome"]), f"{name}: missing expected outcome")
        require(r["implementation"] in {"unassessed", "absent", "partial", "complete"}, f"{name}: invalid implementation")
        refs(r["source_evidence"], "source", name, r["implementation"] in {"partial", "complete"})
        ref = r["reference"]
        require(ref["status"] in {"observed", "user_specified", "manual", "inferred", "unknown"}, f"{name}: invalid reference status")
        kind = {"observed": "reference", "user_specified": "user", "manual": "manual", "inferred": "inference"}.get(ref["status"])
        if kind:
            refs(ref["evidence"], kind, name, True)
        else:
            require(not ref["evidence"], f"{name}: unknown reference cannot cite a measurement")
        outcome(r["runtime"], name)
        parity = r["parity"]
        require(parity in {"pending", "partial", "match", "different", "stale"}, f"{name}: invalid parity")
        if parity == "match":
            require(ref["status"] == "observed" and r["runtime"]["status"] == "pass", f"{name}: parity match requires observed reference and passing runtime evidence")
            refs(r["comparison_evidence"], "runtime", name, True)
    if scope["baseline_complete"]:
        require(all(r["implementation"] != "unassessed" for r in requirements), "Baseline incomplete: unassessed implementation")
    gates = data["gates"]
    for key, gate in gates.items():
        outcome(gate, key)
    members = []
    workflow_ids = set()
    require(bool(data["workflows"]), "Empty workflows")
    for w in data["workflows"]:
        require(w["id"] not in workflow_ids, "Duplicate workflow ID")
        workflow_ids.add(w["id"])
        require(type(w["weight"]) is int and w["weight"] in {1, 2, 3}, "Invalid workflow weight")
        require(bool(w["requirements"]), "Empty workflow")
        require(all(i in ids for i in w["requirements"]), "Unknown workflow requirement")
        require(all(i in gates for i in w["gates"]), "Unknown workflow gate")
        members.extend(w["requirements"])
        outcome(w["scenario"], w["id"])
        require(type(w["user_accepted"]) is bool, "user_accepted must be boolean")
        refs(w["acceptance_evidence"], "user", w["id"], w["user_accepted"])
    require(sorted(members) == sorted(ids), "Every requirement must belong to exactly one workflow")
    return data


def scores(data):
    validate(data)
    rows = data["requirements"]
    total = sum(r["weight"] for r in rows)
    index = {r["id"]: r for r in rows}
    ratio = lambda predicate: 100 * sum(r["weight"] for r in rows if predicate(r)) / total
    ready = []
    for w in data["workflows"]:
        if (w["scenario"]["status"] == "pass"
                and all(index[i]["runtime"]["status"] == "pass" for i in w["requirements"])
                and all(data["gates"][i]["status"] == "pass" for i in w["gates"])):
            ready.append(w["id"])
    completeness = {"unassessed": 0, "absent": 0, "partial": 0.5, "complete": 1}
    return {
        "implementation": (100 * sum(r["weight"] * completeness[r["implementation"]] for r in rows) / total
                           if data["scope"]["baseline_complete"] else None),
        "unassessed": ratio(lambda r: r["implementation"] == "unassessed"),
        "runtime": ratio(lambda r: r["runtime"]["status"] == "pass"),
        "parity": ratio(lambda r: r["parity"] == "match"),
        "unknown": ratio(lambda r: r["reference"]["status"] != "observed"),
        "readiness": 100 * sum(w["weight"] for w in data["workflows"] if w["id"] in ready) / sum(w["weight"] for w in data["workflows"]),
        "ready": ready,
    }


def render(data):
    s = scores(data)
    impl = "Not established (baseline incomplete)" if s["implementation"] is None else f'{s["implementation"]:.1f}%'
    lines = ["# Generated parity report", "", f'Scope: **{data["scope"]["version"]}** — {data["scope"]["description"]}',
             "", "Generated by `python scripts/parity_report.py`; edit ledger.json, not this report.",
             "", "These are documented results for this scope, not a whole-DAW percentage. Zero verified coverage does not prove the feature is absent.", "",
             "| Metric | Result |", "| --- | --- |", f"| Implementation coverage | {impl} |",
             f'| Unassessed implementation weight | {s["unassessed"]:.1f}% |',
             f'| Runtime validation | {s["runtime"]:.1f}% |', f'| Workflow readiness for review | {s["readiness"]:.1f}% |',
             f'| Confirmed reference parity | {s["parity"]:.1f}% |', f'| Unknown/unmeasured reference coverage | {s["unknown"]:.1f}% |',
             f'| Measured reference coverage | {100-s["unknown"]:.1f}% |', "", "## Requirements", "",
             "| ID | Weight | Source | Reference | Runtime | Parity |", "| --- | --- | --- | --- | --- | --- |"]
    for r in data["requirements"]:
        lines.append(f'| {r["id"]} | {r["weight"]} | {r["implementation"]} | {r["reference"]["status"]} | {r["runtime"]["status"]} | {r["parity"]} |')
    lines += ["", "## Essential gates", ""]
    for key, gate in data["gates"].items():
        lines.append(f'- {key}: **{gate["status"]}** — {gate["description"]}')
    lines += ["", "## Workflow review", ""]
    for w in data["workflows"]:
        lines.append(f'- {w["id"]}: scenario {w["scenario"]["status"]}; ready {w["id"] in s["ready"]}; user accepted {w["user_accepted"]}.')
    lines += ["", "Pending/stale/failed checks and unmeasured reference behaviors remain in the denominator. Essential gate failures prevent readiness regardless of averages.", ""]
    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--ledger", type=Path, default=ROOT / "docs/workflow/ledger.json")
    parser.add_argument("--output", type=Path, default=ROOT / "docs/workflow/metrics.md")
    parser.add_argument("--check", action="store_true", help="Validate ledger and require generated report to be current")
    args = parser.parse_args()
    try:
        report = render(json.loads(args.ledger.read_text(encoding="utf-8")))
        if args.check:
            if not args.output.exists() or args.output.read_text(encoding="utf-8") != report:
                raise ValueError("Generated report is missing/stale; run parity_report.py")
        else:
            args.output.write_text(report, encoding="utf-8")
        print(f"Validated {args.ledger.name}; {'checked' if args.check else 'wrote'} {args.output}")
    except (ValueError, KeyError, TypeError, OSError) as error:
        parser.exit(1, f"Parity ledger error: {error}\n")


if __name__ == "__main__":
    main()
