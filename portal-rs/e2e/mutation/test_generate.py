from pathlib import Path
import subprocess
import sys
import unittest

from generate import TAG, attributes, inventory


class InventoryTest(unittest.TestCase):
    def test_template_delimiters_keep_quoted_attributes_together(self):
        match = TAG.search(
            """<form method="post" action="{% if a > b %}/{{ path("en") }}{% endif %}">"""
        )
        self.assertIsNotNone(match)
        self.assertEqual(attributes(match[2]), {
            "method": "post",
            "action": '{% if a > b %}/{{ path("en") }}{% endif %}',
        })

    def test_overlapping_delimiters_do_not_cause_backtracking(self):
        repetitions = 3001
        examples = {
            "tag quotes": "<form " + '""' * repetitions,
            "tag statements": "<form {%" + "%}}{{%" * repetitions,
            "tag expressions": "<form {{" + "}}}}{{{{" * repetitions,
            "attribute expressions": '-="{{{{' + "}}}}{{{{" * repetitions,
            "attribute nested delimiters": '-="' + "{{%%}}" * repetitions,
            "attribute mixed delimiters": '-="{{%' + "%}}{{%" * repetitions,
        }
        # The child deadline also bounds CI if a matcher regresses.
        for name, value in examples.items():
            for match in ("TAG.search(value)", "list(ATTRIBUTE.finditer(value))"):
                with self.subTest(example=name, matcher=match):
                    subprocess.run(
                        [sys.executable, "-B", "-c",
                         f"import sys; from generate import TAG, ATTRIBUTE; value=sys.stdin.read(); {match}"],
                        cwd=Path(__file__).resolve().parent,
                        input=value, text=True, check=True, timeout=1,
                    )

    def test_recovery_key_form_resolves_all_rendered_modes(self):
        generated = inventory()
        [form] = [
            form for form in generated["forms"]
            if form["source"].startswith("crates/identity/src/recovery_key_http.rs:")
        ]
        self.assertEqual(form["actionVariants"], [
            "/{{ context.language }}/account/recovery-key",
            "/{{ context.language }}/recovery-key",
            "/{{ context.language }}/account/recovery-key/confirm",
        ])

        routes = {(row["method"], row["route"]) for row in generated["routes"]}
        for action in form["actionVariants"]:
            with self.subTest(action=action):
                self.assertIn(("POST", action.replace("{{ context.language }}", "{lang}")), routes)


if __name__ == "__main__":
    unittest.main()
