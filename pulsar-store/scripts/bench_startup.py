#!/usr/bin/env python3
"""Time from launching a command to its window appearing (Hyprland only).

    scripts/bench_startup.py [--runs N] -- COMMAND [ARGS...]

The command must create a window of class es.inled.PulsarStore. Runs use a
throwaway HOME so caches start cold. A window appears on screen for each run.
"""
import argparse
import json
import os
import statistics
import subprocess
import sys
import tempfile
import time


def window_pids():
    out = subprocess.run(["hyprctl", "-j", "clients"], capture_output=True, text=True).stdout
    return {c["pid"] for c in json.loads(out) if c.get("class") == "es.inled.PulsarStore"}


def one_run(cmd):
    home = tempfile.mkdtemp(prefix="pulsar-startup-")
    env = {**os.environ, "HOME": home, "XDG_CACHE_HOME": f"{home}/.cache", "XDG_CONFIG_HOME": f"{home}/.config"}
    start = time.perf_counter()
    proc = subprocess.Popen(cmd, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        while time.perf_counter() - start < 20:
            if proc.pid in window_pids():
                return (time.perf_counter() - start) * 1000
            time.sleep(0.02)
        raise SystemExit("window did not appear")
    finally:
        proc.terminate()
        proc.wait()
        time.sleep(0.5)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--runs", type=int, default=5)
    ap.add_argument("cmd", nargs=argparse.REMAINDER)
    args = ap.parse_args()
    cmd = [c for c in args.cmd if c != "--"]
    times = [one_run(cmd) for _ in range(args.runs)]
    print(json.dumps({"runs_ms": [round(t) for t in times], "median_ms": round(statistics.median(times))}))


if __name__ == "__main__":
    sys.exit(main())
