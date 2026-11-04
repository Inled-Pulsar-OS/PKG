"""Seam 4 - extension contract: the status.json parser (model.js), run under gjs."""

import os
import shutil
import subprocess
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))


@unittest.skipUnless(shutil.which("gjs"), "gjs not installed")
class ModelParserTests(unittest.TestCase):
    def test_model_js_meets_its_specification(self):
        result = subprocess.run(["gjs", "-m", os.path.join(HERE, "model_check.js")],
                                capture_output=True, text=True, timeout=30)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)


if __name__ == "__main__":
    unittest.main()
