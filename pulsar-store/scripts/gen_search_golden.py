#!/usr/bin/env python3
# NOTE: the Python store was removed from the tree after the Rust rewrite.
# To run this script, restore it first: git checkout 68f5b074 -- usr/share/pulsar-store
"""Regenerate tests/fixtures/search_golden.json from the Python store's search.

The Rust search must return the same ids in the same order (tests/search_golden.rs).
"""
import json
import os
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(HERE, "..")
QUERIES = ["", " ", "a", "e", "gimp", "GIMP", "plugin", "skill", "sayri", "extension",
           "browser", "image", "code", "notes", "pulsar", "gnome", "ai", "zzzz-no-match"]

home = tempfile.mkdtemp(prefix="pulsar-golden-")
os.environ["HOME"] = home
cache = os.path.join(home, ".cache", "pulsar-store")
os.makedirs(cache)
with open(os.path.join(ROOT, "assets", "catalog", "catalog.json"), encoding="utf-8") as f:
    catalog = json.load(f)
with open(os.path.join(cache, "catalog.json"), "w", encoding="utf-8") as f:
    json.dump(catalog, f)

sys.path.insert(0, os.path.join(ROOT, "usr", "share", "pulsar-store"))
import store_core  # noqa: E402

store_core.StoreCore._prefetch_icons = lambda self: None
core = store_core.StoreCore()
out = {q: [i["id"] for i in core.search(q)] for q in QUERIES}
with open(os.path.join(ROOT, "tests", "fixtures", "search_golden.json"), "w", encoding="utf-8") as f:
    json.dump(out, f, indent=2, ensure_ascii=False)
    f.write("\n")
print(f"wrote {len(out)} queries; {len(core.items)} items")
