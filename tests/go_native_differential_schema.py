"""Go整文件/双制品开发对照，检查EOF锚点及未解决语法差异。"""
import hashlib
import json
import unittest
from zig_aggregate_feedback_schema import ROOT, validator

E = ROOT/'tests/acceptance/evidence'

class GoNativeDifferential(unittest.TestCase):
    def report(self):
        return json.loads((E/'go-native-grammar-differential-2026-10-05.json').read_bytes())

    def test_actual_go_report_keeps_whole_file_scope_native_pair_and_raw_misses(self):
        report=self.report()
        validator('native-grammar-differential-v0.6.schema.json').validate(report)
        self.assertFalse(validator('native-grammar-differential-v0.5.schema.json').is_valid(report))
        data=(E/'go-native-grammar-input-2026-10-05.json').read_bytes()
        validator('grammar-regression-corpus-v0.2.schema.json').validate(json.loads(data))
        self.assertEqual(report['corpus_sha256'],hashlib.sha256(data).hexdigest())
        self.assertEqual((report['sample_count'],report['language_count'],report['selected_language_count']),(20,32,1))
        self.assertEqual(report['grammar_qualified_count'],0)
        self.assertFalse(report['independent_holdout']);self.assertFalse(report['native_adapter_reused']);self.assertTrue(report['program_stable'])
        go=next(r for r in report['languages'] if r['language']=='go')
        self.assertEqual(tuple(go[k] for k in ('tp','fp','fn','tn','native_unknown_count','wasm_unknown_count')),(5,0,2,12,1,0))
        self.assertTrue(go['tool_stable'])
        sources={r['id']:r for r in json.loads(data)['cases']}
        for row in report['cases']:
            source=sources[row['id']]['source'].encode()
            self.assertEqual(row['source_sha256'],hashlib.sha256(source).hexdigest())
            self.assertEqual(row['native']['input_type'],'whole_file')
            validator('go-native-syntax.schema.json').validate(row['native'])
            self.assertEqual(row['comparison'],row['combined_candidate_comparison'])
            if row['id']=='go-logical_line_positions_unresolved':
                self.assertEqual(row['native_classification'],'unknown');continue
            self.assertTrue(row['native_identity_current']);self.assertFalse(row['fixture_native_disagreement'])
            self.assertEqual(row['native']['version'],'go1.23.4')
            self.assertRegex(row['native']['gofmt_sha256'],r'^[0-9a-f]{64}$')
            self.assertRegex(row['native']['companion_binding_sha256'],r'^[0-9a-f]{64}$')
            parts=source.split(b'\n');lines=[p+b'\n' for p in parts[:-1]]
            if parts[-1] or not lines:lines.append(parts[-1])
            for diag in row['native']['diagnostics']:
                original=lines[diag['line']-1];byte=diag['column']-1
                self.assertLessEqual(byte,len(original));original[:byte].decode()
                if byte==len(original):self.assertEqual(diag['line'],len(lines))
        for id in ['go-missing_package_statement','go-missing_package_function']:
            row=next(r for r in report['cases'] if r['id']==id)
            self.assertEqual(row['native_classification'],'invalid');self.assertEqual(row['comparison'],'false_negative')

    def test_eof_mapping_repair_preserves_sources_and_other_classifications(self):
        before=json.loads((E/'go-native-grammar-eof-before-2026-10-05.json').read_bytes())
        validator('native-grammar-differential-v0.6.schema.json').validate(before)
        after=self.report();self.assertEqual(before['corpus_sha256'],after['corpus_sha256'])
        rows={r['id']:r for r in after['cases']}
        for row in before['cases']:
            new=rows[row['id']];self.assertEqual(row['source_sha256'],new['source_sha256'])
            self.assertEqual(row['wasm_classification'],new['wasm_classification'])
            if row['id'] in ['go-missing_brace','go-missing_init','go-missing_paren']:
                self.assertEqual(row['native_classification'],'unknown');self.assertEqual(new['native_classification'],'invalid')
            else:self.assertEqual(row['comparison'],new['comparison'])

    def test_wrong_goal_version_helper_identity_or_claim_is_rejected(self):
        schema=validator('native-grammar-differential-v0.6.schema.json')
        for edit in ['goal','version','rule','column','unit','helper','binding','class','language','qualification','stale_native','stale_program','stale_sdk_summary']:
            report=self.report();row=next(r for r in report['cases'] if r['native']['diagnostics'])
            if edit=='goal':row['native']['input_type']='fragment'
            elif edit=='version':row['native']['version']='go1.23.5'
            elif edit=='rule':row['native']['diagnostics'][0]['rule_id']='gofmt.format'
            elif edit=='column':row['native']['diagnostics'][0]['column']=0
            elif edit=='unit':row['native']['diagnostics'][0]['column_unit']='unicode_scalar'
            elif edit=='helper':row['native']['gofmt_sha256']=None
            elif edit=='binding':row['native']['companion_binding_sha256']=None
            elif edit=='class':row['native_classification']='valid'
            elif edit=='language':row['language']='ruby'
            elif edit=='qualification':report['grammar_qualified_count']=1
            elif edit=='stale_native':row['native_identity_current']=False
            elif edit=='stale_sdk_summary':next(r for r in report['languages'] if r['language']=='go')['tool_stable']=False
            else:report['program_stable']=False
            self.assertFalse(schema.is_valid(report),edit)

    def test_auxiliary_change_preserves_observations_but_withdraws_both_comparisons(self):
        report=self.report()
        next(r for r in report['languages'] if r['language']=='go')['tool_stable']=False
        for row in report['cases']:
            row.update(native_identity_current=False,native_classification='unknown',comparison='unknown',combined_candidate_comparison='unknown',fixture_native_disagreement=None)
        validator('native-grammar-differential-v0.6.schema.json').validate(report)

    def test_actual_eight_language_report_preserves_the_seven_language_corpus(self):
        report=json.loads((E/'native-differential-eight-language-go-2026-10-05.json').read_bytes())
        validator('native-grammar-differential-v0.6.schema.json').validate(report)
        self.assertEqual((report['sample_count'],report['selected_language_count'],report['language_count']),(152,8,32))
        self.assertTrue(report['program_stable']);self.assertEqual(report['grammar_qualified_count'],0);self.assertFalse(report['independent_holdout'])
        selected=[r for r in report['languages'] if r['native_selected']]
        self.assertTrue(all(r['tool_stable'] for r in selected))
        self.assertEqual(tuple(sum(r[k] for r in selected) for k in ('tp','fp','fn','tn')),(39,0,16,91))
        self.assertEqual(sum(r['comparison']=='unknown' for r in report['cases']),6)
        self.assertEqual(sum(not r['native_selected'] for r in report['languages']),24)
        data=(E/'native-differential-eight-language-go-input-2026-10-05.json').read_bytes();self.assertEqual(report['corpus_sha256'],hashlib.sha256(data).hexdigest())
        rows={r['id']:r for r in report['cases']}
        old=json.loads((E/'native-differential-seven-language-ruby-2026-10-05.json').read_bytes())
        for row in old['cases']:
            for key in ['source_sha256','grammar_sha256','fixture_expected_valid','fixture_label','native_classification','wasm_classification','comparison','combined_candidate_classification','combined_candidate_comparison']:
                self.assertEqual(rows[row['id']][key],row[key],(row['id'],key))
        sources={r['id']:r for r in json.loads(data)['cases']}
        for row in report['cases']:self.assertEqual(row['source_sha256'],hashlib.sha256(sources[row['id']]['source'].encode()).hexdigest())

    def test_eight_language_input_retains_every_seven_language_source_and_label(self):
        old=json.loads((E/'native-differential-seven-language-ruby-input-2026-10-05.json').read_bytes())
        new=json.loads((E/'native-differential-eight-language-go-input-2026-10-05.json').read_bytes())
        validator('grammar-regression-corpus-v0.2.schema.json').validate(new)
        self.assertEqual(new['cases'][:len(old['cases'])],old['cases']);self.assertEqual(len(new['cases']),len(old['cases'])+6)

if __name__=='__main__':unittest.main()
