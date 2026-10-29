#!/usr/bin/env python3
"""Fills translations into a .po file from a JSON map of English text to translation.

    scripts/fill-po.py assets/locale/es.po translations.json

Entries named in the map get that text and lose their "fuzzy" mark. Entries with
a context (msgctxt) are skipped, because the same English text can need different
translations in different places. Reports map keys that matched nothing.
"""
import json
import re
import sys

po_path, map_path = sys.argv[1], sys.argv[2]
with open(map_path, encoding="utf-8") as f:
    translations = json.load(f)
with open(po_path, encoding="utf-8") as f:
    text = f.read()

used = set()
blocks = []
for block in text.split("\n\n"):
    m = re.search(r'^msgid "(.*)"$', block, re.M)
    if m and "msgctxt" not in block and m.group(1) in translations:
        key = m.group(1)
        used.add(key)
        block = re.sub(r"^#, fuzzy, ", "#, ", block, flags=re.M)
        block = re.sub(r"^#, fuzzy\n", "", block, flags=re.M)
        value = translations[key].replace("\\", "\\\\").replace('"', '\\"')
        block = re.sub(r'^msgstr ".*"$', f'msgstr "{value}"', block, flags=re.M)
    blocks.append(block)

with open(po_path, "w", encoding="utf-8") as f:
    f.write("\n\n".join(blocks))

missing = sorted(set(translations) - used)
print(f"filled {len(used)} entries")
if missing:
    print("no entry for:", ", ".join(missing))
