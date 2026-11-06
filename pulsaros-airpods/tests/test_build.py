"""Build pipeline contract: one build path for CI and local, deps in one file, one architecture scope."""

import os
import re
import subprocess
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
PKG = os.path.join(HERE, "..")
ROOT = os.path.join(PKG, "..")


def read(*parts):
    with open(os.path.join(ROOT, *parts), encoding="utf-8") as f:
        return f.read()


def deps(distro):
    for line in read("pulsaros-airpods", "build-deps.txt").splitlines():
        if line.startswith(distro + ":"):
            return line.split(":", 1)[1].split()
    return []


class BuildPathTests(unittest.TestCase):
    def test_prepare_assets_has_no_docker_run(self):
        self.assertNotIn("docker run", read("pulsaros-airpods", "prepare-assets.sh"))

    def test_scripts_parse(self):
        for rel in ("pulsaros-airpods/prepare-assets.sh", "pulsaros-airpods/build-local.sh",
                    "install-build-deps.sh", "ci-image.sh"):
            with self.subTest(script=rel):
                subprocess.run(["bash", "-n", os.path.join(ROOT, rel)], check=True)

    def test_qt_and_cmake_deps_are_declared_for_both_distros(self):
        for pkg in ("cmake", "ninja-build", "qt6-base-dev", "qt6-declarative-dev", "qt6-connectivity-dev"):
            self.assertIn(pkg, deps("debian"))
        for pkg in ("cmake", "ninja", "qt6-base", "qt6-declarative", "qt6-connectivity", "qt6-tools"):
            self.assertIn(pkg, deps("arch"))

    def test_ci_and_local_wrapper_install_deps_from_the_same_script(self):
        for rel in (".github/workflows/deploy-package.yml", "pulsaros-airpods/build-local.sh"):
            with self.subTest(file=rel):
                self.assertIn("install-build-deps.sh", read(rel))
        self.assertIn("build-deps.txt", read("install-build-deps.sh"))

    def test_debian_image_follows_the_branch(self):
        wf = read(".github/workflows/deploy-package.yml")
        self.assertNotIn("debian:trixie", wf)
        self.assertIn("ci-image.sh", wf)
        self.assertIn("ci-image.sh", read("pulsaros-airpods/build-local.sh"))
        for branch, image in (("stable", "debian:trixie"), ("unstable", "debian:trixie"),
                              ("forky", "debian:forky"), ("rolling", "debian:testing")):
            with self.subTest(branch=branch):
                out = subprocess.run(["bash", os.path.join(ROOT, "ci-image.sh"), "debian", branch],
                                     check=True, capture_output=True, text=True).stdout.strip()
                self.assertEqual(out, image)

    def test_architecture_scope_is_amd64_only_everywhere(self):
        pkgbuild = read("arch", "pkgbuilds", "pulsaros-airpods", "PKGBUILD")
        self.assertRegex(pkgbuild, r"(?m)^arch=\(x86_64\)$")
        self.assertRegex(read("pulsaros-airpods", "DEBIAN", "control"), r"(?m)^Architecture: amd64$")

    def test_arch_makedepends_cover_the_declared_arch_build_deps(self):
        pkgbuild = read("arch", "pkgbuilds", "pulsaros-airpods", "PKGBUILD")
        declared = " ".join(re.findall(r"(?m)^(?:make)?depends=\((.*?)\)", pkgbuild))
        for pkg in deps("arch"):
            self.assertIn(pkg, declared.split())


if __name__ == "__main__":
    unittest.main()
