"""Evidence and denominator invariants for the progress reporting tool."""
import copy
import json
import unittest

from parity_report import ROOT, scores, validate


class ParityTests(unittest.TestCase):
    def setUp(self):
        self.data = json.loads((ROOT / 'docs/workflow/ledger.json').read_text(encoding='utf-8'))

    def test_unassessed_baseline_suppresses_implementation_percentage(self):
        self.data['scope']['baseline_complete'] = False
        self.data['requirements'][0]['implementation'] = 'unassessed'
        self.assertIsNone(scores(self.data)['implementation'])
        self.data['scope']['baseline_complete'] = True
        with self.assertRaises(ValueError):
            validate(self.data)

    def test_runtime_pass_needs_evidence_identity(self):
        row = self.data['requirements'][0]
        row['runtime'] = {'status': 'pass', 'evidence': []}
        with self.assertRaises(ValueError):
            validate(self.data)
        self.data['evidence']['run'] = {'kind':'runtime', 'location':'test-only', 'identity':''}
        row['runtime']['evidence'] = ['run']
        with self.assertRaises(ValueError):
            validate(self.data)

    def test_user_specification_cannot_establish_reference_parity(self):
        self.data['evidence']['run'] = {'kind':'runtime', 'location':'test-only', 'identity':'fixture-build'}
        row = self.data['requirements'][0]
        row['runtime'] = {'status':'pass', 'evidence':['run']}
        row['parity'] = 'match'
        row['comparison_evidence'] = ['run']
        row['reference'] = {'status':'user_specified','evidence':['intent-2026-10-24']}
        with self.assertRaises(ValueError):
            validate(self.data)

    def test_gates_and_end_to_end_scenario_control_readiness(self):
        self.data['evidence']['run'] = {'kind':'runtime', 'location':'test-only', 'identity':'fixture-build'}
        passing = {'status':'pass', 'evidence':['run']}
        for r in self.data['requirements']:
            r['runtime'] = copy.deepcopy(passing)
        for g in self.data['gates'].values():
            g.update(copy.deepcopy(passing))
        for w in self.data['workflows']:
            w['scenario'] = copy.deepcopy(passing)
        self.assertEqual(scores(self.data)['readiness'], 100)
        gate = self.data['workflows'][0]['gates'][0]
        self.data['gates'][gate]['status'] = 'fail'
        self.assertLess(scores(self.data)['readiness'], 100)
        self.data['gates'][gate]['status'] = 'pass'
        self.data['workflows'][0]['scenario']['status'] = 'stale'
        self.assertLess(scores(self.data)['readiness'], 100)

    def test_unknowns_stay_in_parity_denominator(self):
        for r in self.data['requirements']:
            r['parity'] = 'pending'
            r['reference'] = {'status':'unknown','evidence':[]}
        self.data['evidence']['ref'] = {'kind':'reference','location':'test-only'}
        self.data['evidence']['run'] = {'kind':'runtime','location':'test-only','identity':'fixture-build'}
        row = self.data['requirements'][0]
        row['reference'] = {'status':'observed','evidence':['ref']}
        row['runtime'] = {'status':'pass','evidence':['run']}
        row['parity'] = 'match'
        row['comparison_evidence'] = ['run']
        expected = 100 * row['weight'] / sum(r['weight'] for r in self.data['requirements'])
        self.assertAlmostEqual(scores(self.data)['parity'], expected)
        row['runtime']['status'] = 'stale'
        with self.assertRaises(ValueError):
            validate(self.data)

    def test_workflows_cannot_omit_or_double_count_requirements(self):
        self.data['workflows'][0]['requirements'].pop()
        with self.assertRaises(ValueError):
            validate(self.data)


if __name__ == '__main__':
    unittest.main()
