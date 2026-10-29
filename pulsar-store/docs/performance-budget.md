# Performance baseline and budget

## How the baseline was measured

`scripts/bench_python_baseline.py` loads a real catalog, repeats its entries to reach N items, and times the Python `StoreCore` (no GTK, no network). Run on 2026-10-28, Python 3.14.8, Arch Linux.

The live catalog (`https://store-os.inled.es/schema/index.json`) has **10 packages**. The larger sizes are synthetic and show how the code scales.

| Items | Catalog load (median) | Search (median) | Search (p95) | Peak RSS | Python import |
|---|---|---|---|---|---|
| 10 (real) | 0.25 ms | 0.007 ms | 0.009 ms | 24.9 MB | 38 ms |
| 1,000 | 10.5 ms | 1.5 ms | 2.8 ms | 35.3 MB | 35 ms |
| 5,000 | 55 ms | 3.1 ms | 4.2 ms | 71.2 MB | 37 ms |

## What this tells us

- **Search is not the bottleneck.** Python searches 5,000 items in about 3 ms, and the real catalog has 10. A Rust rewrite will not make search noticeably faster to a user. An earlier draft of the plan promised a 5x search speedup; that target is dropped.
- **Catalog load grows linearly** (about 11 µs per item). It runs once at startup, so it matters only for very large catalogs.
- **The likely costs are in the UI**, which the Python app builds eagerly (every tile at once) and the benchmark does not cover. Those numbers still need measuring.

## Not yet measured

These need a display session and the full app. Record them before ticket 7.x begins, using the same machine for Python and Rust:

- Cold start to first paint of the Python UI
- Time to render the category grid at 10, 1,000, and 5,000 items
- Resident memory of the full GTK app after the first view loads

## Budget for the Rust version

| Metric | Budget | Reason |
|---|---|---|
| Search, 5,000 items, p95 | ≤ 5 ms | Match the Python baseline; users cannot tell the difference |
| Catalog load, 5,000 items | ≤ 25 ms | Roughly half the Python figure; parse cost scales with catalog size |
| `core` binary peak RSS, 5,000 items | ≤ 40 MB | Below the Python figure |
| Time to first paint | Below the Python UI's measured value | Fill in once measured |
| Category grid, 5,000 items | Opens without a visible stall (first frame under 100 ms) | Needs virtualized `GridView` |
| Animation | 60 fps on integrated graphics, none longer than 400 ms | Motion budget |

Ticket 11.4 turns these into a CI check.
