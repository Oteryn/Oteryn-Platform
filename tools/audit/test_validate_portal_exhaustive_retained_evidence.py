import copy
import unittest
from pathlib import Path

from tools.audit.validate_portal_exhaustive_retained_evidence import (
    load_repository_inputs,
    validate_retained_evidence,
)


class RetainedEvidenceProvenanceTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        repo_root = Path(__file__).resolve().parents[2]
        cls.manifest, cls.documents = load_repository_inputs(repo_root)

    def validate(self, manifest=None, documents=None):
        return validate_retained_evidence(
            copy.deepcopy(self.manifest if manifest is None else manifest),
            copy.deepcopy(self.documents if documents is None else documents),
        )

    def assert_error(self, marker: str, manifest=None, documents=None) -> None:
        report = self.validate(manifest, documents)
        self.assertTrue(
            any(marker in error for error in report["errors"]),
            f"Expected error containing {marker!r}; got {report['errors']!r}",
        )

    def test_repository_retained_evidence_is_valid(self) -> None:
        report = self.validate()
        self.assertEqual([], report["errors"])
        self.assertEqual(11, report["retained_document_count"])
        self.assertEqual(
            [
                "67ed852cdd973c9265401190561d968226348649",
                "f5f83b8122fa266bb8f7dc45019fea566ac53fb5",
            ],
            report["observed_embedded_source_shas"],
        )
        self.assertEqual("e4c16048288ba9a9bd699a7c3427495105922503", report["final_pr_head_sha"])
        self.assertEqual("cbbd7613cee13cf01931a0ba0f7ac089122132e0", report["merge_sha"])

    def test_unexplained_embedded_sha_fails(self) -> None:
        documents = copy.deepcopy(self.documents)
        documents.append(("rogue.json", {"exact_sha": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}))
        self.assert_error("unexplained embedded source SHA", documents=documents)

    def test_final_artifact_digest_must_be_sha256(self) -> None:
        manifest = copy.deepcopy(self.manifest)
        manifest["provenance"]["final_pr_head"]["artifact_digest"] = "not-a-digest"
        self.assert_error("final_pr_head artifact_digest must be a sha256 digest", manifest=manifest)

    def test_durable_final_reference_must_match_final_stage(self) -> None:
        manifest = copy.deepcopy(self.manifest)
        manifest["provenance"]["durable_final_exact_head_reference"]["artifact_id"] += 1
        self.assert_error("durable final exact-head reference artifact_id", manifest=manifest)

    def test_merge_and_final_head_must_remain_distinct(self) -> None:
        manifest = copy.deepcopy(self.manifest)
        manifest["provenance"]["merge"]["sha"] = manifest["provenance"]["final_pr_head"]["sha"]
        self.assert_error("must remain distinct", manifest=manifest)

    def test_manifest_source_must_match_strict_stage(self) -> None:
        manifest = copy.deepcopy(self.manifest)
        manifest["source_sha"] = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
        self.assert_error("manifest source_sha must equal provenance.strict_source.sha", manifest=manifest)


if __name__ == "__main__":
    unittest.main()
