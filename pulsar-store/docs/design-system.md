# Design system

Pulsar Store takes inspiration from Apple's Liquid Glass design (see `apple-reference.md`) and builds it on GTK4 and libadwaita. It copies ideas and structure, not Apple artwork, icons, or fonts.

![Discover, dark](screenshots/discover-dark.png)
![Discover, light](screenshots/discover-light.png)

## Layers

- **Content layer:** pages, lists, cards. Flat, no glass.
- **Navigation layer:** the floating toolbar at the top. This is the only glass surface. Never put glass on content, and never stack glass on glass.

## Tokens

Numeric tokens live in `src/ui/tokens.rs`. Colors live in `assets/css/light.css` and `assets/css/dark.css` as `@store_*` names, built on libadwaita's named colors so the system accent color and light/dark setting apply. A test checks that both theme files define every token and that components use only defined ones.

| Group | Names |
|---|---|
| Spacing | `SPACE_XS` 4, `SPACE_S` 8, `SPACE_M` 12, `SPACE_L` 16, `SPACE_XL` 24, `SPACE_XXL` 32 |
| Radii | window 18, card 18, tile 14, icon 14, pill 999 |
| Colors | `store_surface`, `store_surface_hover`, `store_hairline`, `store_text_dim`, `store_surface_flat`, `store_glass_tint`, `store_glass_edge`, `store_glass_edge_dim`, `store_glass_shadow`, `store_hero_scrim`, `store_accent_text` |

Controls inside a rounded surface use concentric radii: `concentric(outer, inset)` returns `outer - inset`.

## Type

The app uses the system font (libadwaita's default, Inter on most Pulsar OS installs). No proprietary font is bundled. Titles use weight 800, section titles 700, captions 12 px in the dim text color.

## Glass

`GlassSurface` (`src/ui/glass.rs`) blurs the content behind it, adds a tint, a lit top edge, and a soft shadow. GTK cannot bend light, so this approximates Liquid Glass without true lensing. It falls back to a flat translucent surface when high contrast is on or `PULSAR_STORE_FLAT_GLASS=1` is set.

## Widgets

| Widget | File | Notes |
|---|---|---|
| Item tile | `ui/widget/item_tile.rs` | Icon, name, summary, install button. Click or Enter opens the item. |
| Card | `ui/widget/card.rs` | Larger; the hero variant uses the accent gradient. |
| Install button | `ui/widget/install_button.rs` | Get, progress ring, Open, Update. Tap the ring to cancel. |
| Progress ring | `ui/widget/progress_ring.rs` | Determinate, or spinning when unknown. |
| Info grid | `ui/widget/info_grid.rs` | Value over label cells. |
| Item grid | `ui/widget/item_grid.rs` | Virtualized; builds only visible rows. |

![Item page](screenshots/item-dark.png)

## Motion

Durations: 150 ms fast, 250 ms base, 350 ms slow, never over 400 ms. Pages fade in; the navigation view slides pages. Tiles and cards dim slightly while pressed. Everything respects the system "animations" setting (`gtk-enable-animations`).

## Regenerating screenshots

`scripts/screenshot-all.sh` opens each page in the real app on a Hyprland session and saves images to `docs/screenshots/`. Review the diff by eye.
