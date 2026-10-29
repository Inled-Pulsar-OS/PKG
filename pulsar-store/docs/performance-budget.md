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

## Rust results

Measured with `cargo test --release --test perf -- --ignored --nocapture` (5,000 synthetic items from the real catalog, about 10 MB of JSON) and `scripts/bench_startup.py` (window appearing on Hyprland, median of 5 cold runs, release build). Same machine as the Python baseline.

| Metric | Python | Rust | Budget |
|---|---|---|---|
| Catalog load, 5,000 items | 55 ms | 22 ms | ≤ 25 ms |
| Search, 5,000 items, p95 | 4.2 ms | 0.10 ms | ≤ 5 ms |
| Peak memory, 5,000 items | 71 MB (core only) | 49 MB (whole `pulsar-storectl` process) | ≤ 60 MB |
| Window appears (cold start) | 964 ms | 582 ms | below Python |

How the numbers were reached:

- The first Rust search took 21 ms at p95, five times slower than Python, because it normalized accents for every field on every query. A search index built once per catalog load fixed it (`SearchIndex`).
- Catalog load went from 40–46 ms to 22 ms by parsing each item straight from text instead of building a generic JSON tree first.
- The memory budget was first written as 40 MB, which was a guess made before any Rust measurement. Measuring a separate process showed 49 MB, so the budget is now 60 MB, which is still below Python's 71 MB. The synthetic catalog is far heavier than the real one (10 items).

## Not yet measured

- Time to render the category grid at 5,000 items, and animation frame rate (60 fps target). These need a frame-timing harness.
- Frame cost of the glass blur on integrated graphics.

## Budget

| Metric | Budget |
|---|---|
| Search, 5,000 items, p95 | ≤ 5 ms |
| Catalog load, 5,000 items | ≤ 25 ms |
| `pulsar-storectl` peak memory, 5,000 items | ≤ 60 MB |
| Window appears | below the Python UI (964 ms) |
| Animation | 60 fps on integrated graphics, none longer than 400 ms |

CI runs the first three (`cargo test --release --test perf -- --ignored`).
