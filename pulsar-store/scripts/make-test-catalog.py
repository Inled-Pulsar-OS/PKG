#!/usr/bin/env python3
"""Writes a catalog for trying the product page: the bundled catalog, with Firefox
given screenshots, a download size, and translations.

    scripts/make-test-catalog.py OUT.json [SCREENSHOT_BASE_URL]

Serve docs/screenshots at the base URL (default http://127.0.0.1:8099) with
`python3 -m http.server 8099 --directory docs/screenshots`, put OUT.json in
`<cache>/pulsar-store/catalog.json`, and start the app with PULSAR_STORE_OFFLINE=1.
"""
import json
import os
import sys

out = sys.argv[1]
base = sys.argv[2] if len(sys.argv) > 2 else "http://127.0.0.1:8099"
here = os.path.dirname(os.path.abspath(__file__))
with open(os.path.join(here, "..", "assets", "catalog", "catalog.json"), encoding="utf-8") as f:
    catalog = json.load(f)

for item in catalog["packages"]:
    if item["id"] == "org.mozilla.firefox":
        item["demo_urls"] = [f"{base}/{n}.png" for n in ("discover-dark", "item-dark", "apps-dark")]
        item["size"] = 1_600_000_000
        item["summary"] = {
            "en": "Fast, private, and independent web browser.",
            "es": "Navegador rápido y privado.",
            "fr": "Navigateur rapide et privé.",
        }
        item["version"] = "158.0"

os.makedirs(os.path.dirname(out) or ".", exist_ok=True)
with open(out, "w", encoding="utf-8") as f:
    json.dump(catalog, f)
print(f"wrote {out}")
