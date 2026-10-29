#!/usr/bin/env python3
"""Rewrites the index in docs/tickets/README.md from the ticket files.

Each ticket shows its status: done (all acceptance boxes ticked), partial, or open.
"""
import glob
import os
import re

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "docs", "tickets")
BLOCKS = {}


def acceptance(text):
    section = re.search(r"## Acceptance criteria\n(.*?)\n## ", text, re.S)
    boxes = re.findall(r"- \[( |x)\]", section.group(1)) if section else []
    return boxes


def main():
    rows = {}
    for path in sorted(glob.glob(os.path.join(ROOT, "b*-*", "*.md"))):
        text = open(path, encoding="utf-8").read()
        head = re.match(r"# (\S+) (.*)", text)
        block = re.search(r"\*\*Block:\*\* (B\d+) (.*)", text)
        size = re.search(r"\*\*Size:\*\* (\w)", text)
        deps = re.search(r"\*\*Depends on:\*\* (.*)", text)
        boxes = acceptance(text)
        done = boxes.count("x")
        status = "done" if boxes and done == len(boxes) else ("partial" if done else "open")
        if "won't do" in head.group(2):
            status = "closed"
        rel = os.path.relpath(path, ROOT)
        key = int(block.group(1)[1:])
        BLOCKS[key] = f"{block.group(1)} {block.group(2)}"
        rows.setdefault(key, []).append((head.group(1), head.group(2), size.group(1), deps.group(1), status, rel))

    def sort_id(r):
        return [int(p) for p in r[0].split(".")]

    out = ["## Index\n", "Status: **done** = every acceptance box ticked, **partial** = some ticked (the ticket says what is left), **open**, **closed** = decided not to do.\n"]
    counts = {}
    for key in sorted(rows):
        out.append(f"\n### {BLOCKS[key]}\n\n")
        for i, title, size, deps, status, rel in sorted(rows[key], key=sort_id):
            counts[status] = counts.get(status, 0) + 1
            dep = "" if deps == "none" else f" — depends on {deps}"
            out.append(f"- [{i}]({rel}) {title} ({size}) **{status}**{dep}\n")
    summary = ", ".join(f"{n} {s}" for s, n in sorted(counts.items()))
    readme = os.path.join(ROOT, "README.md")
    text = open(readme, encoding="utf-8").read()
    text = text[: text.index("## Index")] + "".join(out)
    text = text.replace("## Index\n", f"## Index\n\nTotals: {summary}.\n", 1)
    open(readme, "w", encoding="utf-8").write(text)
    print(summary)


if __name__ == "__main__":
    main()
