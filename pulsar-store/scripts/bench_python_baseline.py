#!/usr/bin/env python3
# NOTE: the Python store was removed from the tree after the Rust rewrite.
# To run this script, restore it first: git checkout 68f5b074 -- usr/share/pulsar-store
"""Baseline for the Python store core (no GTK, no network).

Measures import time, catalog load time, search latency, and peak RSS using
a synthetic catalog built by repeating a real catalog's entries.

    scripts/bench_python_baseline.py --catalog index.json --items 5000
"""
import argparse
import copy
import json
import os
import resource
import statistics
import sys
import tempfile
import time

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "usr", "share", "pulsar-store")
QUERIES = ["gimp", "edit", "image", "browser", "code", "game", "music", "zzzz-no-match", "a", "pulsar"]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--catalog", required=True, help="path to a real catalog JSON")
    ap.add_argument("--items", type=int, default=5000)
    ap.add_argument("--runs", type=int, default=5)
    args = ap.parse_args()

    with open(args.catalog, encoding="utf-8") as f:
        base = json.load(f)
    pkgs = base.get("packages") or base.get("items") or []
    big = []
    for i in range(args.items):
        p = copy.deepcopy(pkgs[i % len(pkgs)])
        p["id"] = f"{p['id']}.{i}"
        big.append(p)

    home = tempfile.mkdtemp(prefix="pulsar-bench-")
    os.environ["HOME"] = home
    cache = os.path.join(home, ".cache", "pulsar-store")
    os.makedirs(cache)
    with open(os.path.join(cache, "catalog.json"), "w", encoding="utf-8") as f:
        json.dump({"packages": big}, f)

    sys.path.insert(0, ROOT)
    t = time.perf_counter()
    import store_core  # noqa: E402
    import_s = time.perf_counter() - t

    store_core.StoreCore._prefetch_icons = lambda self: None  # no network

    loads = []
    for _ in range(args.runs):
        t = time.perf_counter()
        core = store_core.StoreCore()
        loads.append(time.perf_counter() - t)

    lat = []
    for _ in range(args.runs):
        for q in QUERIES:
            t = time.perf_counter()
            core.search(q)
            lat.append(time.perf_counter() - t)
    lat.sort()

    out = {
        "items": len(core.items),
        "import_ms": round(import_s * 1000, 2),
        "catalog_load_ms_median": round(statistics.median(loads) * 1000, 2),
        "search_ms_median": round(statistics.median(lat) * 1000, 3),
        "search_ms_p95": round(lat[int(len(lat) * 0.95) - 1] * 1000, 3),
        "peak_rss_mb": round(resource.getrusage(resource.RUSAGE_SELF).ru_maxrss / 1024, 1),
        "python": sys.version.split()[0],
    }
    print(json.dumps(out, indent=2))


if __name__ == "__main__":
    main()
