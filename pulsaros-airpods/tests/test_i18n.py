"""Seam 4 - extension contract: every user-visible string is translatable and translated.

Catalogs are JSON files in gnome-shell-extension/i18n/<lang>.json: {English string: translation}.
"""

import glob
import json
import os
import re
import shutil
import subprocess
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
EXT = os.path.join(HERE, "..", "gnome-shell-extension")
CATALOGS = sorted(glob.glob(os.path.join(EXT, "i18n", "*.json")))


def js_without_comments():
    with open(os.path.join(EXT, "indicator.js"), encoding="utf-8") as f:
        return re.sub(r"//[^\n]*", "", f.read())


def source_msgids():
    """Strings the extension passes to _()."""
    found = re.findall(r"\b_\('((?:[^'\\]|\\.)*)'\)", js_without_comments())
    return {m.replace("\\'", "'") for m in found}


def placeholders(text):
    return sorted(re.findall(r"\{\w+\}", text))


class WrappedLiteralTests(unittest.TestCase):
    # Patterns that put a bare English literal on screen. Each one must go through _().
    BARE = {
        "menu item": r"(?:PopupMenuItem|PopupSwitchMenuItem|PopupSubMenuMenuItem)\(\s*'[A-Za-z]",
        "text assignment": r"(?:\.text|text)\s*[:=]\s*'[A-Za-z]",
        "ternary result": r"[?:]\s*'[A-Z][a-z]+(?: [a-z]+)*'",
        "returned label": r"\breturn\s+'[A-Z]",
        "row title": r"\bmakeRow\([^)]*'[A-Z]",
        "notification": r"Main\.notify\(\s*'",
    }

    def test_no_bare_user_visible_literals(self):
        source = js_without_comments()
        for name, pattern in self.BARE.items():
            with self.subTest(kind=name):
                self.assertEqual(re.findall(pattern, source), [], f"unwrapped {name}")


class CatalogTests(unittest.TestCase):
    def test_spanish_catalog_exists(self):
        self.assertIn("es.json", [os.path.basename(p) for p in CATALOGS])

    def test_every_catalog_translates_exactly_the_source_strings(self):
        source = source_msgids()
        self.assertGreater(len(source), 5)  # the regex found the _() calls
        for path in CATALOGS:
            with self.subTest(catalog=os.path.basename(path)):
                with open(path, encoding="utf-8") as f:
                    catalog = json.load(f)
                self.assertEqual(set(catalog), source)
                for msgid, text in catalog.items():
                    self.assertTrue(text.strip(), f"empty translation of {msgid!r}")
                    self.assertEqual(placeholders(msgid), placeholders(text), msgid)


@unittest.skipUnless(shutil.which("gjs"), "gjs not installed")
class LoaderTests(unittest.TestCase):
    def test_loader_picks_the_language_and_falls_back_to_english(self):
        result = subprocess.run(["gjs", "-m", os.path.join(HERE, "i18n_check.js")],
                                capture_output=True, text=True, timeout=30)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)


if __name__ == "__main__":
    unittest.main()
