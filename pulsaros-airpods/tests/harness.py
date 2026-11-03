"""Shared helpers for the tests that need a private D-Bus session bus.

Run those tests with:
    PULSAROS_TEST_BUS=1 dbus-run-session -- python3 -m unittest discover -s tests
They are skipped otherwise, so they never touch the user's real session bus.
"""

import json
import os
import subprocess
import sys
import tempfile
import threading
import time
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(HERE, "..", "src")
sys.path.insert(0, HERE)
sys.path.insert(0, ROOT)

requires_private_bus = unittest.skipUnless(
    os.environ.get("DBUS_SESSION_BUS_ADDRESS") and os.environ.get("PULSAROS_TEST_BUS") == "1",
    "run under: PULSAROS_TEST_BUS=1 dbus-run-session -- python3 -m unittest discover -s tests",
)


def pump(condition, timeout=10.0):
    """Run the default GLib main context until condition() is true. Returns the result."""
    from gi.repository import GLib

    ctx = GLib.MainContext.default()
    end = time.time() + timeout
    while time.time() < end:
        while ctx.iteration(False):
            pass
        if condition():
            return True
        time.sleep(0.02)
    return bool(condition())


def settle(seconds):
    """Pump the main context for a fixed time (to prove that something does NOT happen)."""
    pump(lambda: False, timeout=seconds)


class FakeBluez:
    """tests/fake_bluez.py as a subprocess; stdout lines after "ready" are collected."""

    def __init__(self):
        self.lines = []
        self.proc = None
        self.start()

    def start(self):
        self.proc = subprocess.Popen(
            [sys.executable, os.path.join(HERE, "fake_bluez.py")],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            text=True,
            bufsize=1,
        )
        assert self.proc.stdout.readline().strip() == "ready"
        threading.Thread(target=self._drain, args=(self.proc,), daemon=True).start()

    def _drain(self, proc):
        for line in proc.stdout:
            self.lines.append(line.strip())

    def count(self, prefix):
        return sum(1 for line in self.lines if line.startswith(prefix))

    def send(self, **cmd):
        self.proc.stdin.write(json.dumps(cmd) + "\n")
        self.proc.stdin.flush()

    def stop(self):
        if self.proc is None:
            return
        self.proc.terminate()
        try:
            self.proc.wait(5)
        except subprocess.TimeoutExpired:
            self.proc.kill()
        for stream in (self.proc.stdin, self.proc.stdout):
            try:
                stream.close()
            except OSError:
                pass
        self.proc = None


class Daemon:
    def __init__(self, config_path):
        self.config_path = config_path
        self.proc = None
        self.log_path = config_path + ".log"
        self.start()

    def start(self):
        env = dict(os.environ, PYTHONPATH=os.path.abspath(ROOT))
        self._log = open(self.log_path, "ab")
        self.proc = subprocess.Popen(
            [sys.executable, "-m", "pulsaros_airpods.daemon", "--bluez-bus", "session",
             "--config", self.config_path, "--debug"],
            env=env,
            stderr=self._log,
        )

    def alive(self):
        return self.proc is not None and self.proc.poll() is None

    def stop(self):
        if self.proc is None:
            return
        self.proc.terminate()
        try:
            self.proc.wait(5)
        except subprocess.TimeoutExpired:
            self.proc.kill()
        self._log.close()
        self.proc = None

    def log(self):
        try:
            with open(self.log_path, errors="replace") as f:
                return f.read()[-3000:]
        except OSError:
            return ""


def tempdir():
    return tempfile.TemporaryDirectory()
