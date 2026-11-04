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


    def test_status_source_follows_the_file(self):
        result = subprocess.run(["gjs", "-m", os.path.join(HERE, "status_source_check.js")],
                                capture_output=True, text=True, timeout=60)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)


    def run_gjs(self, script):
        result = subprocess.run(["gjs", "-m", os.path.join(HERE, script)],
                                capture_output=True, text=True, timeout=60)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_optimistic_state_holds_a_click_until_the_daemon_agrees(self):
        self.run_gjs("optimistic_check.js")

    def test_control_runner_spawns_only_whitelisted_verbs(self):
        self.run_gjs("ctl_check.js")


if __name__ == "__main__":
    unittest.main()
