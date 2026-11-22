import unittest

from generate import inventory


class InventoryTest(unittest.TestCase):
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
