#!/usr/bin/env python3
"""Focused regressions for the Platform consumer of central META policy."""

from __future__ import annotations

import html
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import unittest
from urllib.parse import urlparse

from policy_consistency import (
    BINDING_PATH,
    CATALOG_PATH,
    REPO_ROOT,
    PolicyConsistencyError,
    authenticate_meta_checkout,
    validate_policy,
)

PUBLICATION_INTEGRITY_AUTHORITY = "1bfb5ff98c8aa156e73669a14e083a1d464c29fb"


class PolicyConsistencyTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        configured = os.environ.get("OTERYN_META_POLICY_ROOT")
        if not configured:
            raise RuntimeError("OTERYN_META_POLICY_ROOT must name the exact bound META checkout")
        cls.meta_root = Path(configured).resolve()
        cls.binding = json.loads((REPO_ROOT / BINDING_PATH).read_text(encoding="utf-8"))
        cls.authority = authenticate_meta_checkout(
            cls.meta_root,
            cls.binding,
            github_json=cls._fake_github(cls.meta_root, cls.binding),
        )

    @staticmethod
    def _fake_github(meta_root: Path, binding: dict[str, object], *, corrupt_path: str | None = None):
        commit = str(binding["authority_commit"])
        tree = []
        output = subprocess.run(
            ["git", "ls-tree", "-r", "HEAD"],
            cwd=meta_root,
            check=True,
            capture_output=True,
            text=True,
        ).stdout
        for line in output.splitlines():
            metadata, path = line.split("\t", 1)
            _mode, kind, sha = metadata.split()
            if path == corrupt_path:
                sha = "0" * 40
            tree.append({"path": path, "type": kind, "sha": sha})
        main = "f" * 40

        def read(url: str) -> object:
            path = urlparse(url).path
            if path.endswith(f"/commits/{commit}"):
                return {"sha": commit}
            if path.endswith("/branches/main"):
                return {"protected": True, "commit": {"sha": main}}
            if "/compare/" in path:
                return {
                    "status": "ahead",
                    "base_commit": {"sha": commit},
                    "merge_base_commit": {"sha": commit},
                }
            if "/git/trees/" in path:
                return {"truncated": False, "tree": tree}
            raise AssertionError(f"unexpected fake GitHub URL: {url}")

        return read

    def _fixture(self) -> tuple[tempfile.TemporaryDirectory[str], Path]:
        temporary = tempfile.TemporaryDirectory()
        root = Path(temporary.name)
        for relative in (Path("AGENTS.md"), BINDING_PATH, CATALOG_PATH):
            target = root / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(REPO_ROOT / relative, target)
        shutil.copytree(REPO_ROOT / "docs/agents/prompts", root / "docs/agents/prompts")
        return temporary, root

    def _findings(self, root: Path) -> str:
        return "\n".join(validate_policy(root, self.meta_root, self.authority))

    @staticmethod
    def _bound_meta_checkout_target(workflow: str) -> tuple[str, str]:
        match = re.search(
            r"(?ms)^\s*- name: Check out bound META policy\s*$"
            r"(?P<body>.*?)(?=^\s*- name:|\Z)",
            workflow,
        )
        if match is None:
            raise AssertionError("bound META checkout step is missing")
        active = "\n".join(
            line for line in match.group("body").splitlines()
            if not line.lstrip().startswith("#")
        )
        repository = re.search(r"(?m)^\s*repository:\s*([^\s#]+)\s*$", active)
        ref = re.search(r"(?m)^\s*ref:\s*([0-9a-f]{40})\s*$", active)
        if repository is None or ref is None:
            raise AssertionError("bound META checkout step lacks one active repository/ref")
        return repository.group(1), ref.group(1)

    @staticmethod
    def _consume_subsequence(text: str, word: str, start: int) -> int | None:
        cursor = start
        lowered = text.casefold()
        for character in word.casefold():
            position = lowered.find(character, cursor)
            if position < 0:
                return None
            cursor = position + 1
        return cursor

    @classmethod
    def _authority_heading_source_matches(cls, text: str) -> bool:
        # Deliberately fail closed for this single protected heading. CommonMark
        # inline syntax can split visible words with emphasis, links, references,
        # destinations and entities. Strip only truly non-rendered comments/tags,
        # then require the authority words as ordered character subsequences.
        # False positives are acceptable here: they stop governance validation
        # rather than allowing an ambiguous duplicate authority section through.
        candidate = html.unescape(text)
        candidate = re.sub(r"<!--.*?-->", "", candidate, flags=re.DOTALL)
        candidate = re.sub(r"</?[A-Za-z][^>]*>", "", candidate)

        cursor = 0
        for word in ("GitHub", "credential", "compatibility"):
            next_cursor = cls._consume_subsequence(candidate, word, cursor)
            if next_cursor is None:
                return False
            cursor = next_cursor
        return True

    @staticmethod
    def _markdown_html_block_start(line: str) -> bool:
        # Only classify constructs that can actually open a CommonMark HTML block.
        # Inline HTML with rendered text (for example
        # <span>GitHub credential compatibility</span>) must remain paragraph
        # content so a following Setext underline can make it a heading.
        if re.match(
            r"^ {0,3}(?:"
            r"<(?:script|pre|style|textarea)(?:[ \t>]|$)|"
            r"<!--|<\?|<![A-Z]|<!\[CDATA\["
            r")",
            line,
            re.IGNORECASE,
        ):
            return True
        block_tags = (
            "address|article|aside|base|basefont|blockquote|body|caption|center|col|"
            "colgroup|dd|details|dialog|dir|div|dl|dt|fieldset|figcaption|figure|"
            "footer|form|frame|frameset|h[1-6]|head|header|hr|html|iframe|legend|"
            "li|link|main|menu|menuitem|nav|noframes|ol|optgroup|option|p|param|"
            "search|section|summary|table|tbody|td|tfoot|th|thead|title|tr|track|ul"
        )
        if re.match(
            rf"^ {{0,3}}</?(?:{block_tags})(?:[ \t]+|/?>|$)",
            line,
            re.IGNORECASE,
        ):
            return True
        # Type-7 HTML blocks require a complete standalone tag line.
        # A tag carrying inline rendered text is deliberately not matched.
        return re.fullmatch(
            r" {0,3}</?[A-Za-z][A-Za-z0-9-]*(?:[ \t]+[^<>\n]*)?/?"
            r">[ \t]*",
            line,
        ) is not None

    @classmethod
    def _markdown_other_block_start(cls, line: str) -> bool:
        if not line:
            return False
        leading = re.match(r"^[ \t]*", line)
        prefix = leading.group(0) if leading is not None else ""
        if "\t" in prefix:
            return True
        if len(prefix) >= 4:
            return True
        if re.match(
            r"^ {0,3}(?:"
            r">(?:[ \t]+|$)|"
            r"(?:[-+*]|\d+[.)])(?:[ \t]+|$)"
            r")",
            line,
        ):
            return True
        if cls._markdown_html_block_start(line):
            return True
        return re.fullmatch(
            r" {0,3}(?:(?:\*[ \t]*){3,}|(?:_[ \t]*){3,}|(?:-[ \t]*){3,})",
            line,
        ) is not None

    @classmethod
    def _markdown_h2_spans(cls, markdown: str) -> list[tuple[int, int, str]]:
        lines = markdown.splitlines(keepends=True)
        offsets: list[int] = []
        cursor = 0
        for line in lines:
            offsets.append(cursor)
            cursor += len(line)

        spans: list[tuple[int, int, str]] = []
        fence: tuple[str, int] | None = None
        paragraph_start: int | None = None

        for index, line in enumerate(lines):
            stripped = line.rstrip("\r\n")

            if re.match(r"^[ \t]*\t[ \t]*(?:\x60{3,}|~{3,})", stripped):
                raise AssertionError("ambiguous tab-indented fence line")

            if fence is not None:
                marker, width = fence
                if re.fullmatch(
                    rf" {{0,3}}{re.escape(marker)}{{{width},}}[ \t]*",
                    stripped,
                ):
                    fence = None
                paragraph_start = None
                continue

            fence_open = re.match(r"^ {0,3}(\x60{3,}|~{3,})(.*)$", stripped)
            if fence_open is not None:
                marker = fence_open.group(1)
                remainder = fence_open.group(2)
                if marker[0] == "\x60" and "\x60" in remainder:
                    raise AssertionError("invalid backtick fence opener")
                fence = (marker[0], len(marker))
                paragraph_start = None
                continue

            if not stripped.strip():
                paragraph_start = None
                continue

            if re.match(r"^[ \t]*\t[ \t]*##(?:[ \t]+|$)", stripped):
                raise AssertionError("ambiguous tab-indented ATX heading")

            atx = re.match(
                r"^ {0,3}##(?:[ \t]+|$)(?P<title>.*)$",
                stripped,
            )
            if atx is not None:
                title = re.sub(r"[ \t]+#+[ \t]*$", "", atx.group("title"))
                spans.append(
                    (
                        offsets[index],
                        offsets[index] + len(line),
                        title,
                    )
                )
                paragraph_start = None
                continue

            if re.match(r"^[ \t]*\t[ \t]*-+[ \t]*$", stripped):
                raise AssertionError("ambiguous tab-indented Setext underline")

            if re.fullmatch(r" {0,3}-+[ \t]*", stripped):
                if paragraph_start is not None and paragraph_start < index:
                    title = " ".join(
                        part.rstrip("\r\n").strip()
                        for part in lines[paragraph_start:index]
                    )
                    spans.append(
                        (
                            offsets[paragraph_start],
                            offsets[index] + len(line),
                            title,
                        )
                    )
                paragraph_start = None
                continue

            leading = re.match(r"^[ \t]*", stripped)
            prefix = leading.group(0) if leading is not None else ""
            if paragraph_start is not None and ("\t" in prefix or len(prefix) >= 4):
                continue

            if cls._markdown_other_block_start(stripped):
                paragraph_start = None
                continue

            if paragraph_start is None:
                paragraph_start = index

        return spans

    @classmethod
    def _credential_compatibility_section(cls, bootstrap: str) -> str:
        headings = cls._markdown_h2_spans(bootstrap)
        matches = [
            (start, end)
            for start, end, title in headings
            if cls._authority_heading_source_matches(title)
        ]
        if len(matches) != 1:
            raise AssertionError(
                "bootstrap must contain exactly one GitHub credential compatibility "
                f"section, found {len(matches)}"
            )

        start = matches[0][1]
        next_h2 = min(
            (
                heading_start
                for heading_start, _heading_end, _title in headings
                if heading_start >= start
            ),
            default=None,
        )
        end = next_h2 if next_h2 is not None else len(bootstrap)
        return bootstrap[start:end].strip()

    @classmethod
    def _publication_fallback_clause(cls, bootstrap: str) -> str:
        section = cls._credential_compatibility_section(bootstrap)
        matches = re.findall(
            r"(?ms)^If an already-prepared material candidate cannot use the normal "
            r"authorized publication path, .*?(?=\n\n|\Z)",
            section,
        )
        if len(matches) != 1:
            raise AssertionError(
                "GitHub credential compatibility must contain exactly one operative "
                f"publication fallback clause, found {len(matches)}"
            )
        return matches[0]

    @staticmethod
    def _credential_compatibility_section_is_bound(section: str) -> bool:
        expected = (
            'This compatibility path applies only to an already-authorized existing task '
            'branch and PR; PR creation remains a coordinator/control-plane action. If '
            '`GH_TOKEN` and `GITHUB_TOKEN` are unset but agent-visible `GH` exists, it may '
            'be passed transiently as `GH_TOKEN="$GH"` to the exact authorized `gh` command. '
            'Do not assume that mapping authenticates `git push`, embed a token in a remote '
            'URL, or persist a credential helper. Use another authorized repository-native '
            'write path when the existing Git transport cannot consume the identity.\n\n'
            'If an already-prepared material candidate cannot use the normal authorized '
            'publication path, do not silently reconstruct or relabel that selected candidate. '
            'Preserve its custody and return publication control to the active control plane. '
            'The control plane may select only an API-native **new candidate** route permitted '
            'by the bound META policy, including the bounded connector-compatible Git Data mode '
            'only under its exact one-writer/predecessor/one-commit/non-force/post-readback '
            'conditions. Ad-hoc raw Git Data reconstruction, sequential per-file API publication, '
            'force/ref replacement, reset and rebase remain forbidden.\n\n'
            'Credential presence grants no repository, branch, path, merge, production or secret '
            'authority. Never force-push, and verify the remote exact head after publication.'
        )
        normalize = lambda value: re.sub(r"[ \t]+", " ", value).strip()
        return normalize(section) == normalize(expected)

    @staticmethod
    def _publication_fallback_clause_is_bound(clause: str) -> bool:
        expected = (
            "If an already-prepared material candidate cannot use the normal authorized "
            "publication path, do not silently reconstruct or relabel that selected candidate. "
            "Preserve its custody and return publication control to the active control plane. "
            "The control plane may select only an API-native **new candidate** route permitted "
            "by the bound META policy, including the bounded connector-compatible Git Data mode "
            "only under its exact one-writer/predecessor/one-commit/non-force/post-readback "
            "conditions. Ad-hoc raw Git Data reconstruction, sequential per-file API publication, "
            "force/ref replacement, reset and rebase remain forbidden."
        )
        normalize = lambda value: re.sub(r"\s+", " ", value).strip()
        return normalize(clause) == normalize(expected)

    def test_current_repository_adopts_authenticated_central_policy(self) -> None:
        self.assertEqual([], validate_policy(REPO_ROOT, self.meta_root, self.authority))

    def test_publication_integrity_authority_and_fallback_are_bound(self) -> None:
        self.assertEqual(self.binding["authority_commit"], PUBLICATION_INTEGRITY_AUTHORITY)

        workflow = (REPO_ROOT / ".github/workflows/agent-governance.yml").read_text(encoding="utf-8")
        self.assertEqual(
            ("Oteryn/Oteryn", PUBLICATION_INTEGRITY_AUTHORITY),
            self._bound_meta_checkout_target(workflow),
        )
        stale_workflow = workflow.replace(
            f"          ref: {PUBLICATION_INTEGRITY_AUTHORITY}",
            "          # stale evidence only: ref: "
            f"{PUBLICATION_INTEGRITY_AUTHORITY}\n"
            "          ref: 33b212e652c680bd4047be3b414c9a358b8bf26f",
            1,
        )
        self.assertEqual(
            ("Oteryn/Oteryn", "33b212e652c680bd4047be3b414c9a358b8bf26f"),
            self._bound_meta_checkout_target(stale_workflow),
        )

        bootstrap = (REPO_ROOT / "docs/agents/PLATFORM_AGENT_BOOTSTRAP.md").read_text(encoding="utf-8")
        section = self._credential_compatibility_section(bootstrap)
        self.assertTrue(self._credential_compatibility_section_is_bound(section))
        clause = self._publication_fallback_clause(bootstrap)
        self.assertTrue(self._publication_fallback_clause_is_bound(clause))
        historical_prefix = clause + "\n\nHistorical evidence only.\n\n"
        relaxed_live = bootstrap.replace(
            clause,
            clause.replace(
                "Preserve its custody and return publication control to the active control plane",
                "Preserve its custody",
                1,
            ),
            1,
        )
        self.assertFalse(
            self._publication_fallback_clause_is_bound(
                self._publication_fallback_clause(historical_prefix + relaxed_live)
            ),
            "a historical copy before the live section must not qualify a relaxed live clause",
        )
        duplicate_inside_live_section = bootstrap.replace(
            "## GitHub credential compatibility\n\n",
            "## GitHub credential compatibility\n\n"
            + clause
            + "\n\nHistorical evidence only.\n\n",
            1,
        )
        with self.assertRaisesRegex(AssertionError, "exactly one operative"):
            self._publication_fallback_clause(duplicate_inside_live_section)

        duplicate_section = (
            "## GitHub credential compatibility\n\n"
            + section
            + "\n\n"
            + bootstrap
        )
        with self.assertRaisesRegex(AssertionError, "exactly one GitHub credential compatibility"):
            self._credential_compatibility_section(duplicate_section)

        equivalent_duplicate_section = (
            "## GitHub credential compatibility ##\n\n"
            + section
            + "\n\n"
            + bootstrap
        )
        with self.assertRaisesRegex(AssertionError, "exactly one GitHub credential compatibility"):
            self._credential_compatibility_section(equivalent_duplicate_section)

        setext_duplicate_section = (
            "GitHub credential compatibility\n---\n\n"
            + section
            + "\n\n"
            + bootstrap
        )
        with self.assertRaisesRegex(AssertionError, "exactly one GitHub credential compatibility"):
            self._credential_compatibility_section(setext_duplicate_section)

        short_setext_duplicate_section = (
            "GitHub credential compatibility\n-\n\n"
            + section
            + "\n\n"
            + bootstrap
        )
        with self.assertRaisesRegex(AssertionError, "exactly one GitHub credential compatibility"):
            self._credential_compatibility_section(short_setext_duplicate_section)

        inline_html_setext_duplicate = (
            "<span>GitHub credential compatibility</span>\n---\n\n"
            + section
            + "\n\n"
            + bootstrap
        )
        with self.assertRaisesRegex(AssertionError, "exactly one GitHub credential compatibility"):
            self._credential_compatibility_section(inline_html_setext_duplicate)

        wrapped_setext_duplicate_section = (
            "GitHub credential\ncompatibility\n---\n\n"
            + section
            + "\n\n"
            + bootstrap
        )
        with self.assertRaisesRegex(AssertionError, "exactly one GitHub credential compatibility"):
            self._credential_compatibility_section(wrapped_setext_duplicate_section)

        wrapped_setext_after_atx_block = (
            "## Historical evidence\n"
            "GitHub credential\ncompatibility\n---\n\n"
            + section
            + "\n\n"
            + bootstrap
        )
        with self.assertRaisesRegex(AssertionError, "exactly one GitHub credential compatibility"):
            self._credential_compatibility_section(wrapped_setext_after_atx_block)

        fenced_setext_example = (
            "```text\n"
            "GitHub credential\n"
            "compatibility\n---\n"
            "```\n\n"
            + bootstrap
        )
        self.assertEqual(
            section,
            self._credential_compatibility_section(fenced_setext_example),
        )

        inline_markup_duplicate = (
            "GitHub **credential**\n"
            "compatibility\n---\n\n"
            + section
            + "\n\n"
            + bootstrap
        )
        with self.assertRaisesRegex(AssertionError, "exactly one GitHub credential compatibility"):
            self._credential_compatibility_section(inline_markup_duplicate)

        inline_link_duplicate = (
            "GitHub [credential](https://example.invalid) compatibility\n"
            "---\n\n"
            + section
            + "\n\n"
            + bootstrap
        )
        with self.assertRaisesRegex(AssertionError, "exactly one GitHub credential compatibility"):
            self._credential_compatibility_section(inline_link_duplicate)

        invalid_backtick_fence = (
            "```foo`bar\n"
            "GitHub credential compatibility\n"
            "---\n\n"
            "```\n"
            + bootstrap
        )
        with self.assertRaisesRegex(AssertionError, "invalid backtick fence opener"):
            self._credential_compatibility_section(invalid_backtick_fence)

        entity_duplicate_section = (
            "## GitHub credential compatibilit&#121;\n\n"
            + section
            + "\n\n"
            + bootstrap
        )
        with self.assertRaisesRegex(AssertionError, "exactly one GitHub credential compatibility"):
            self._credential_compatibility_section(entity_duplicate_section)

        tab_indented_pseudo_fence = (
            "\t```foo\n"
            "GitHub credential compatibility\n"
            "---\n\n"
            "```\n"
            + bootstrap
        )
        with self.assertRaisesRegex(AssertionError, "ambiguous tab-indented fence line"):
            self._credential_compatibility_section(tab_indented_pseudo_fence)

        shortcut_reference_duplicate = (
            "## GitHub [credential] compatibility\n\n"
            "[credential]: /target\n\n"
            + section
            + "\n\n"
            + bootstrap
        )
        with self.assertRaisesRegex(AssertionError, "exactly one GitHub credential compatibility"):
            self._credential_compatibility_section(shortcut_reference_duplicate)

        inline_comment_duplicate = (
            "## GitHub cred<!--hidden-->ential compatibility\n\n"
            + section
            + "\n\n"
            + bootstrap
        )
        with self.assertRaisesRegex(AssertionError, "exactly one GitHub credential compatibility"):
            self._credential_compatibility_section(inline_comment_duplicate)

        inline_emphasis_split_word = (
            "## GitHub cred**ential** compatibility\n\n"
            + section
            + "\n\n"
            + bootstrap
        )
        with self.assertRaisesRegex(AssertionError, "exactly one GitHub credential compatibility"):
            self._credential_compatibility_section(inline_emphasis_split_word)

        inline_underscore_split_word = (
            "## GitHub cred__ential__ compatibility\n\n"
            + section
            + "\n\n"
            + bootstrap
        )
        with self.assertRaisesRegex(AssertionError, "exactly one GitHub credential compatibility"):
            self._credential_compatibility_section(inline_underscore_split_word)

        inline_link_label_split_word = (
            "## GitHub cred[ential](https://example.invalid/foo(bar)) compatibility\n\n"
            + section
            + "\n\n"
            + bootstrap
        )
        with self.assertRaisesRegex(AssertionError, "exactly one GitHub credential compatibility"):
            self._credential_compatibility_section(inline_link_label_split_word)

        inline_shortcut_label_split_word = (
            "## GitHub cred[ential] compatibility\n\n"
            "[ential]: /target\n\n"
            + section
            + "\n\n"
            + bootstrap
        )
        with self.assertRaisesRegex(AssertionError, "exactly one GitHub credential compatibility"):
            self._credential_compatibility_section(inline_shortcut_label_split_word)

        inline_full_reference_split_word = (
            "## GitHub cred[ential][target] compatibility\n\n"
            "[target]: /destination\n\n"
            + section
            + "\n\n"
            + bootstrap
        )
        with self.assertRaisesRegex(AssertionError, "exactly one GitHub credential compatibility"):
            self._credential_compatibility_section(inline_full_reference_split_word)

        inline_collapsed_reference_split_word = (
            "## GitHub cred[ential][] compatibility\n\n"
            "[ential]: /destination\n\n"
            + section
            + "\n\n"
            + bootstrap
        )
        with self.assertRaisesRegex(AssertionError, "exactly one GitHub credential compatibility"):
            self._credential_compatibility_section(inline_collapsed_reference_split_word)

        inline_destination_inside_word = (
            "## GitHub cred[ent](https://example.invalid/foo(bar))ial compatibility\n\n"
            + section
            + "\n\n"
            + bootstrap
        )
        with self.assertRaisesRegex(AssertionError, "exactly one GitHub credential compatibility"):
            self._credential_compatibility_section(inline_destination_inside_word)

        heavily_split_authority_heading = (
            "## G*i*tH[u](https://example.invalid)b cr[e][target]d__ent__ial compatibilit&#121;\n\n"
            "[target]: /destination\n\n"
            + section
            + "\n\n"
            + bootstrap
        )
        with self.assertRaisesRegex(AssertionError, "exactly one GitHub credential compatibility"):
            self._credential_compatibility_section(heavily_split_authority_heading)

        indented_setext_paragraph_continuation = (
            "GitHub\n"
            "    credential\n"
            "compatibility\n"
            "---\n\n"
            + section
            + "\n\n"
            + bootstrap
        )
        with self.assertRaisesRegex(AssertionError, "exactly one GitHub credential compatibility"):
            self._credential_compatibility_section(indented_setext_paragraph_continuation)

        short_setext_single_hyphen = (
            "GitHub credential compatibility\n"
            "-\n\n"
            + section
            + "\n\n"
            + bootstrap
        )
        with self.assertRaisesRegex(AssertionError, "exactly one GitHub credential compatibility"):
            self._credential_compatibility_section(short_setext_single_hyphen)

        short_setext_two_hyphens = (
            "GitHub credential compatibility\n"
            "--\n\n"
            + section
            + "\n\n"
            + bootstrap
        )
        with self.assertRaisesRegex(AssertionError, "exactly one GitHub credential compatibility"):
            self._credential_compatibility_section(short_setext_two_hyphens)

        inline_html_setext_title = (
            "<span>GitHub credential compatibility</span>\n"
            "---\n\n"
            + section
            + "\n\n"
            + bootstrap
        )
        with self.assertRaisesRegex(AssertionError, "exactly one GitHub credential compatibility"):
            self._credential_compatibility_section(inline_html_setext_title)

        tab_indented_pseudo_closer = (
            "```\n"
            "\t```\n"
            + bootstrap
            + "\n```\n"
            "## GitHub credential compatibility\n\n"
            "Relaxed historical text.\n"
        )
        with self.assertRaisesRegex(AssertionError, "ambiguous tab-indented fence line"):
            self._credential_compatibility_section(tab_indented_pseudo_closer)

        balanced_destination_duplicate = (
            "## GitHub [credential](https://example.invalid/foo(bar)) compatibility\n\n"
            + section
            + "\n\n"
            + bootstrap
        )
        with self.assertRaisesRegex(AssertionError, "exactly one GitHub credential compatibility"):
            self._credential_compatibility_section(balanced_destination_duplicate)

        tab_indented_atx = (
            "\t## GitHub credential compatibility\n\n"
            + bootstrap
        )
        with self.assertRaisesRegex(AssertionError, "ambiguous tab-indented ATX heading"):
            self._credential_compatibility_section(tab_indented_atx)

        tab_indented_setext = (
            "GitHub credential compatibility\n"
            "\t---\n\n"
            + bootstrap
        )
        with self.assertRaisesRegex(AssertionError, "ambiguous tab-indented Setext underline"):
            self._credential_compatibility_section(tab_indented_setext)

        wrapped_setext_next_section = bootstrap.replace(
            section,
            section + "\n\nAnother wrapped\nsection\n---\n\nHistorical evidence only.",
            1,
        )
        self.assertEqual(
            section,
            self._credential_compatibility_section(wrapped_setext_next_section),
        )

        standalone_permission = bootstrap.replace(
            clause,
            clause + "\n\nReset and rebase may be used for recovery.",
            1,
        )
        mutated_section = self._credential_compatibility_section(standalone_permission)
        self.assertFalse(self._credential_compatibility_section_is_bound(mutated_section))
        for required in (
            "do not silently reconstruct or relabel that selected candidate",
            "Preserve its custody and return publication control to the active control plane",
            "The control plane may select only an API-native **new candidate** route permitted by the bound META policy",
            "bounded connector-compatible Git Data mode only under its exact one-writer/predecessor/one-commit/non-force/post-readback conditions",
            "Ad-hoc raw Git Data reconstruction",
            "sequential per-file API publication",
            "force/ref replacement",
            "reset and rebase remain forbidden",
        ):
            self.assertFalse(
                self._publication_fallback_clause_is_bound(clause.replace(required, "", 1)),
                required,
            )
        for additive_contradiction in (
            " Selected candidate may be reconstructed for recovery.",
            " Publication control may remain with the worker during recovery.",
            " Ad-hoc raw Git Data reconstruction is allowed for recovery.",
            " Ad-hoc raw Git Data reconstruction may be used for recovery.",
            " Sequential per-file API publication is permitted for recovery.",
            " Sequential per-file API publication may be used for recovery.",
            " Force/ref replacement is authorized for recovery.",
            " Force/ref replacement may be used for recovery.",
            " Reset and rebase are allowed for recovery.",
            " Reset and rebase may be used for recovery.",
            " The worker may use reset when recovery is difficult.",
        ):
            self.assertFalse(
                self._publication_fallback_clause_is_bound(
                    clause + additive_contradiction
                ),
                additive_contradiction,
            )
        contradictory = bootstrap.replace(
            clause,
            clause + " Reset and rebase are allowed for recovery.",
            1,
        ) + "\n\nHistorical note: reset and rebase remain forbidden.\n"
        self.assertFalse(
            self._publication_fallback_clause_is_bound(
                self._publication_fallback_clause(contradictory)
            )
        )

        contract = (self.meta_root / "docs/agents/contracts/PUBLICATION_INTEGRITY_POLICY.md").read_text(encoding="utf-8")
        for value in (
            "bounded connector-compatible Git Data route",
            "Either API route creates a **new candidate**",
            "GitHub Git-refs `force=false` proves only ancestry",
            "sequential per-file commits",
            "partial/mixed reconstruction remain forbidden",
        ):
            self.assertIn(value, contract)

    def test_authentication_rejects_dirty_executable_policy_checkout(self) -> None:
        temporary = tempfile.TemporaryDirectory(); self.addCleanup(temporary.cleanup)
        clone = Path(temporary.name) / "meta"
        subprocess.run(["git", "clone", "--quiet", "--no-hardlinks", str(self.meta_root), str(clone)], check=True)
        path = clone / "tools/governance/central_agent_policy.py"
        path.write_text(path.read_text(encoding="utf-8") + "\n# tampered\n", encoding="utf-8")
        with self.assertRaisesRegex(PolicyConsistencyError, "must be clean"):
            authenticate_meta_checkout(clone, self.binding, github_json=self._fake_github(clone, self.binding))

    def test_authentication_rejects_blob_mismatch_before_code_load(self) -> None:
        with self.assertRaisesRegex(PolicyConsistencyError, "does not match authenticated GitHub blob"):
            authenticate_meta_checkout(
                self.meta_root,
                self.binding,
                github_json=self._fake_github(
                    self.meta_root,
                    self.binding,
                    corrupt_path="tools/governance/central_agent_policy.py",
                ),
            )

    def test_missing_binding_fails_closed(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        (root / BINDING_PATH).unlink()
        self.assertIn("cannot read valid JSON", self._findings(root))

    def test_binding_version_drift_fails_closed(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        path = root / BINDING_PATH
        binding = json.loads(path.read_text(encoding="utf-8"))
        binding["policy_version"] = "999.0.0"
        path.write_text(json.dumps(binding), encoding="utf-8")
        self.assertIn("policy_version", self._findings(root))

    def test_binding_commit_must_match_authenticated_authority(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        path = root / BINDING_PATH
        binding = json.loads(path.read_text(encoding="utf-8"))
        binding["authority_commit"] = "0" * 40
        path.write_text(json.dumps(binding), encoding="utf-8")
        self.assertIn("could not be resolved", self._findings(root))

    def test_root_must_resolve_binding(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        path = root / "AGENTS.md"
        path.write_text(path.read_text(encoding="utf-8").replace("Resolve `docs/agents/META_AGENT_POLICY_BINDING.json`", "Mention `docs/agents/META_AGENT_POLICY_BINDING.json`", 1), encoding="utf-8")
        self.assertIn("provider overlay must resolve", self._findings(root))

    def test_root_parallel_first_policy_is_rejected_by_central_validator(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        path = root / "AGENTS.md"
        path.write_text(path.read_text(encoding="utf-8") + "\nAgents must use parallel-first execution.\n", encoding="utf-8")
        self.assertIn("parallel-first execution wording is forbidden", self._findings(root))

    def test_active_prompt_cannot_embed_global_execution_policy(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        catalog = json.loads((root / CATALOG_PATH).read_text(encoding="utf-8"))
        active = next(entry for entry in catalog["prompts"] if entry["executable"] is True)
        path = root / active["path"]
        path.write_text(path.read_text(encoding="utf-8") + "\n## GitHub-first execution\n", encoding="utf-8")
        self.assertIn("task prompt must not copy organization-wide policy sections", self._findings(root))

    def test_historical_prompt_body_is_not_an_active_policy_consumer(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        catalog = json.loads((root / CATALOG_PATH).read_text(encoding="utf-8"))
        historical = next(entry for entry in catalog["prompts"] if entry["executable"] is False)
        path = root / historical["path"]
        path.write_text(path.read_text(encoding="utf-8") + "\n## GitHub-first execution\n", encoding="utf-8")
        self.assertEqual([], validate_policy(root, self.meta_root, self.authority))

    def test_executable_prompt_requires_active_reusable_classification(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        path = root / CATALOG_PATH
        catalog = json.loads(path.read_text(encoding="utf-8"))
        active = next(entry for entry in catalog["prompts"] if entry["executable"] is True)
        active["status"] = "historical_do_not_run"
        path.write_text(json.dumps(catalog), encoding="utf-8")
        self.assertIn("executable prompt must be reusable/active_reusable", self._findings(root))

    def test_historical_prompt_requires_inert_lifecycle(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        path = root / CATALOG_PATH
        catalog = json.loads(path.read_text(encoding="utf-8"))
        historical = next(entry for entry in catalog["prompts"] if entry["executable"] is False)
        historical["classification"] = "reusable"
        path.write_text(json.dumps(catalog), encoding="utf-8")
        self.assertIn("inert prompt must be one_shot_historical/historical_do_not_run", self._findings(root))

    def test_duplicate_prompt_path_fails_closed(self) -> None:
        temporary, root = self._fixture(); self.addCleanup(temporary.cleanup)
        path = root / CATALOG_PATH
        catalog = json.loads(path.read_text(encoding="utf-8"))
        catalog["prompts"].append(dict(catalog["prompts"][0]))
        path.write_text(json.dumps(catalog), encoding="utf-8")
        self.assertIn("duplicate prompt path", self._findings(root))


if __name__ == "__main__":
    unittest.main(verbosity=2)
