#!/usr/bin/env python3
"""Generate data/emoji.tsv from a local Unicode emoji-test.txt (never fetched at runtime).

usage: gen-emoji.py EMOJI_TEST_TXT [EMOJILIB_EMOJIS_JSON] > data/emoji.tsv
columns: emoji <TAB> name <TAB> keywords <TAB> group <TAB> skin-tone variants (space separated)
Keywords: sub-group words plus optional emojilib keywords. Skin tone variants fold under the base.
"""
import json, re, sys

lib = {}
if len(sys.argv) > 2:
    for v in json.load(open(sys.argv[2], encoding="utf-8")).values():
        lib[v["char"].replace("️", "")] = v["keywords"]

rows, order = {}, []
group = sub = ""
for line in open(sys.argv[1], encoding="utf-8"):
    if line.startswith("# group:"):
        group = line.split(":", 1)[1].strip()
    elif line.startswith("# subgroup:"):
        sub = line.split(":", 1)[1].strip().replace("-", " ")
    m = re.match(r"[0-9A-F ]+;\s*fully-qualified\s*# (\S+) E[\d.]+ (.*)", line)
    if not m or group == "Component":
        continue
    emo, name = m.groups()
    if "skin tone" in name:
        base = name.split(": ", 1)[0].strip()
        if base in rows:
            rows[base][4].append(emo)
        continue
    kws = sorted(set(sub.split()) | set(lib.get(emo.replace("️", ""), [])))
    rows[name] = [emo, name, " ".join(k.replace("_", " ") for k in kws), group, []]
    order.append(name)
for n in order:
    r = rows[n]
    print("\t".join(r[:4] + [" ".join(r[4])]))
