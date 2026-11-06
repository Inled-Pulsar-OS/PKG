"""Extension contract: guards that only show up inside the Shell (a missing import, stale metadata)."""

import json
import os
import re
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
EXT = os.path.join(HERE, "..", "gnome-shell-extension")


class ExtensionContractTests(unittest.TestCase):
    """Guards that only show up inside the Shell: a missing import or a stale metadata file."""

    def js(self, name):
        with open(os.path.join(EXT, name), encoding="utf-8") as f:
            return f.read()

    def test_every_local_import_resolves_to_a_file(self):
        for name in sorted(n for n in os.listdir(EXT) if n.endswith(".js")):
            for target in re.findall(r"from '\./([\w.-]+)'", self.js(name)):
                with self.subTest(importer=name, target=target):
                    self.assertTrue(os.path.exists(os.path.join(EXT, target)), target)

    def test_indicator_no_longer_uses_the_v1_dbus_service(self):
        source = self.js("indicator.js")
        self.assertNotIn("interface.js", source)
        self.assertNotIn("org.pulsaros", source)
        self.assertIn("StatusSource", source)

    def test_metadata_targets_48_and_51_and_matches_directory_name(self):
        with open(os.path.join(EXT, "metadata.json")) as f:
            meta = json.load(f)
        self.assertEqual(meta["uuid"], "airpods@pulsaros.org")  # the installed directory name
        self.assertEqual(meta["shell-version"], ["48", "51"])

    def test_every_icon_the_indicator_asks_for_ships(self):
        names = set(re.findall(r"'(airpods[\w-]*-symbolic)'", self.js("indicator.js")))
        self.assertTrue(names)
        for name in names:
            self.assertTrue(os.path.exists(os.path.join(EXT, "icons", name + ".svg")), name)


if __name__ == "__main__":
    unittest.main()
