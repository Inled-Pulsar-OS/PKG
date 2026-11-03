"""interface.js (embedded in the extension) must match the D-Bus contract XML."""

import os
import re
import unittest

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")
XML = os.path.join(ROOT, "data", "org.pulsaros.AirPods.xml")
JS = os.path.join(ROOT, "gnome-shell-extension", "interface.js")


def members(text):
    props = re.findall(r'<property name="(\w+)" type="(\w+)" access="(\w+)"', text)
    signals = re.findall(r'<signal name="(\w+)"', text)
    return props, signals


class ExtensionContractTests(unittest.TestCase):
    def test_interface_js_matches_xml(self):
        with open(XML, encoding="utf-8") as f:
            xml = f.read()
        with open(JS, encoding="utf-8") as f:
            js = f.read()
        self.assertEqual(members(xml), members(js))
        self.assertGreater(len(members(xml)[0]), 15)


if __name__ == "__main__":
    unittest.main()
