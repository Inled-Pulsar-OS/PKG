#!/usr/bin/env python3
"""Which CSS selectors match in the real GNOME 48 Settings window (trixie container)?

Every selector in probe-selectors.txt gets its own colour, loaded as the user stylesheet
(~/.config/gtk-4.0/gtk.css) of the real app; several panels are screenshotted and the exact
pixels of each colour are counted. 0 pixels in every view = no visible widget matched.

  scripts/probe-selectors.py [workdir]     needs docker and Pillow

Line forms in probe-selectors.txt:  <selector>  |  RAW <css with @C@ = the colour>
Colours avoid every colour of an unprobed baseline render. opacity:1 because dim labels are translucent (blended, never the exact colour). The fill is for small labels (an
outline on a fractional pixel never has the exact colour), the outline for containers whose children
cover the fill. Two selectors that hit the same
widget fight over the same colour (the more specific one wins), so the run repeats with only
the selectors that have not matched yet, until a pass finds nothing new.
"""
import glob, os, shutil, subprocess, sys, tempfile
from collections import Counter
from PIL import Image

HERE = os.path.dirname(os.path.realpath(__file__))
WORK = os.path.realpath(sys.argv[1] if len(sys.argv) > 1 else tempfile.mkdtemp())
# sidebar rows at x=100; the sidebar scrolls, so the lower panels come after "scrolled"
VIEWS = ";".join(f"{n}:mousemove 100 {y} click 1" for n, y in [
    ("multitasking", 358), ("sound", 278), ("power", 318), ("displays", 238),
    ("apps", 451), ("notifications", 491)]) + \
    ";scrolled:mousemove 100 500" + " click 5" * 8 + ";" + ";".join(
    f"{n}:mousemove 100 {y} click 1" for n, y in [
        ("mouse", 359), ("keyboard", 400), ("printers", 479),
        ("accessibility", 532), ("privacy", 572), ("system", 612)])
lines = [l.strip() for l in open(HERE + "/probe-selectors.txt") if l.strip() and l[0] != "#"]


def render(name, css=None):
    d = f"{WORK}/{name}"
    os.makedirs(d, exist_ok=True)
    debs = glob.glob(f"{WORK}/base/*.deb")  # built once, by the baseline run
    for deb in debs:
        if os.path.dirname(deb) != d:
            shutil.copy(deb, d)
    env = dict(os.environ, WORK_DIR=d, VIEWS=VIEWS, PANEL_CLICK="100 398", SKIP_BUILD="1" if debs else "")
    if css:
        open(d + "/probe.css", "w").write(css)
        env["PROBE_CSS"] = d + "/probe.css"
    subprocess.run([HERE + "/render-settings-trixie.sh", d + "/appearance.png"], env=env, check=True,
                   stdout=open(d + "/run.log", "w"), stderr=subprocess.STDOUT)
    return {os.path.basename(f)[:-4].replace("appearance.", ""): Counter(Image.open(f).convert("RGB").getdata())
            for f in sorted(glob.glob(f"{d}/*.png"))}


base = render("base")
used = set().union(*base.values())
pal = ((r, g, b) for i in range(1, 100000) for r, g, b in [((i * 53) % 241 + 10, (i * 97) % 241 + 10, (i * 151) % 241 + 10)])
colors = []
for c in pal:
    if c not in used and all(sum(abs(a - b) for a, b in zip(c, o)) > 60 for o in colors):
        colors.append(c)
    if len(colors) >= len(lines):
        break

hits = {}
todo = list(range(len(lines)))
for p in range(6):
    css = ""
    for i in todo:
        col = "rgb(%d,%d,%d)" % colors[i]
        if lines[i].startswith("RAW "):
            css += lines[i][4:].replace("@C@", col) + "\n"
        else:
            css += "%s { opacity: 1; background-color: %s; outline: 3px solid %s; outline-offset: -3px; }\n" % (lines[i], col, col)
    shots = render(f"pass{p}", css)
    found = {i: {v: c[colors[i]] for v, c in shots.items() if c[colors[i]] >= 8} for i in todo}
    new = [i for i in todo if found[i]]
    hits.update({i: found[i] for i in new})
    todo = [i for i in todo if i not in new]
    if not new:
        break

for i, line in enumerate(lines):
    h = hits.get(i)
    print("%-100s %s" % (line[:100], ("MATCH " + ",".join(f"{k}={v}" for k, v in sorted(h.items()))) if h else "no match"))
