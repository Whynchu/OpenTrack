"""Opt-in installed effect compatibility sweep; each class runs in a bounded process.

Run from the repository root: python scripts/test_vst3_installed.py
Native editors are shown briefly. Instruments are reported separately because
insert hosting has no MIDI/instrument routing. This is not production sandboxing.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--folder", default=r"C:\Program Files\Common Files\VST3")
    parser.add_argument("--output", type=Path, default=Path("builds/x64/vst3-compatibility.json"))
    parser.add_argument("--timeout", type=int, default=45)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    os.chdir(root)
    output = args.output.resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    inventory_path = output.with_name(output.stem + "-inventory.json")
    subprocess.run(["cargo", "run", "-p", "opentrack-plugin-scan", "--", "--enumerate",
                    "--no-default-folders", "--folder", args.folder, "--json", "--output", str(inventory_path)], check=True)
    build = subprocess.run(["cargo", "test", "-p", "opentrack", "--no-run", "--message-format=json"],
                           check=True, capture_output=True, text=True)
    executable = next(item["executable"] for line in build.stdout.splitlines()
                      if (item := json.loads(line)).get("reason") == "compiler-artifact"
                      and item.get("executable") and item.get("profile", {}).get("test")
                      and item["target"]["name"] == "opentrack")
    binary_sha256 = hashlib.sha256(Path(executable).read_bytes()).hexdigest()
    source_sha256 = {
        str(path.relative_to(root)): hashlib.sha256(path.read_bytes()).hexdigest()
        for folder in ("crates/opentrack-vst3/src", "crates/opentrack-app/src")
        for path in (root / folder).glob("*.rs")
    }
    inventory = json.loads(inventory_path.read_text(encoding="utf-8-sig"))
    results = []
    for plugin in inventory["plugins"]:
        result = {key: plugin[key] for key in ("name", "class_id", "module_path", "category")}
        if plugin["category"] == "instrument":
            result.update(status="unsupported_instrument_routing")
        else:
            env = os.environ.copy()
            env.pop("OPENTRACK_VST3_PROBE_VIEW_SECONDS", None)
            env.update(OPENTRACK_VST3_PROBE_MODULE=plugin["module_path"],
                       OPENTRACK_VST3_PROBE_CLASS=plugin["class_id"])
            try:
                run = subprocess.run([executable, "installed_class_editor_lifecycle", "--ignored",
                                      "--nocapture", "--test-threads=1"], env=env,
                                     capture_output=True, text=True, timeout=args.timeout)
                result.update(status="passed" if run.returncode == 0 else "failed",
                              exit_code=run.returncode, log=run.stdout + run.stderr)
            except subprocess.TimeoutExpired as error:
                result.update(status="timeout", log=str(error.stdout) + str(error.stderr))
        results.append(result)
        output.write_text(json.dumps({"executable": executable, "executable_sha256": binary_sha256,
                                      "source_sha256": source_sha256, "results": results}, indent=2), encoding="utf-8")
        print(f"{result['status']}: {plugin['name']}", flush=True)
    return int(any(r["status"] in ("failed", "timeout") for r in results))


if __name__ == "__main__":
    sys.exit(main())
