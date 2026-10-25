# Plan: match macOS 26 (Tahoe) Spotlight

References (not copied into the repo): `~/Pictures/spotlight/Spotlight-Applications.jpg`,
`Spotlight-Clipboard.jpg`, `Spotlight-in-macOS-26-Tahoe.webp`.

This replaces the Raycast-style visual layer built earlier. The behavior from the oma-cast work
(ranking, frecency, sections, Ctrl+K actions, calculator, shortcut table) stays. Only the look
and the mode model change.

## 1. What the references show

**Shell**
- One large frosted-glass panel, corner radius about 28-32 px, pale tinted fill (periwinkle over the blue wallpaper), 1 px light edge, soft large drop shadow. No title bar.
- No footer bar. No visible selection highlight in the screenshots (selection is subtle).
- Roughly 640 px wide in a 900 px reference frame, so about 0.7 of the screen height is never used: the panel grows with content and stops at about 560 px tall, then clips with a fade.

**Header**
- Mode glyph on the left (App Store "A" for Applications, document-copy for Clipboard, magnifier when idle), about 28 px.
- Large light-weight input text, about 30 px, the mode name shown as placeholder text ("Applications", "Clipboard", "Spotlight Search").
- Circular `...` button on the right (opens options), about 32 px.
- Hairline divider below the header, inset 20 px each side.

**Applications mode**
- Row of pill chips (Utilities, Social, Productivity & Finance, ...) that scrolls horizontally, translucent fill, muted text.
- Icon grid, 5 columns, about 60 px squircle icons, label centered below, one line, ellipsis. The first row is "top apps" and a hairline separates it from the rest (alphabetical).

**Clipboard mode**
- Rows about 56 px tall: left item icon with a small source-app badge, title in 17 px dark text (ellipsis at the end), second line muted `Text · Copied 4:19 PM` or `URL · host · Copied ...`.
- Right: circular copy button (translucent disc, glyph inside).

**Tahoe idle state (webp)**
- Pill-shaped search bar (full radius) plus four circular glass buttons to its right: Applications, Files, Actions, Clipboard. These are the mode switchers. Glass has a specular top highlight and a colored soft shadow.

## 2. What the current app has

- `data/base.css` (376 lines) styles the Raycast layout: 750x480 window, 14 px radius, divider, section headers, 28 px icon tiles, footer bar.
- `src/ui/window.rs` has header (`search-header`), `category_chip` filter (All, Applications, Clipboard, Files...), result list, footer, action panel overlay.
- Grid mode was removed in the Raycast pass; `is_grid_view` is unused. Needs to come back for Applications.
- Liquid glass is only detected through the GNOME extension `liquid-glass@thinkingcoding1231.gmail.com`, and only changes alpha values.

## 3. Hard constraint: blur

GTK4 cannot blur what is behind its own window. Options:
1. **Translucent fill plus gradient and edge light, no blur** (works everywhere, looks flatter).
2. **Compositor blur**: Hyprland `windowrule` for class `es.inled.pulsaros-spotlight` (Omarchy, your screenshot machine) or a GNOME blur extension (Pulsar's liquid-glass extension, or Blur my Shell).

Plan: build option 1 so it looks right alone, then ship a documented blur rule for Hyprland and keep the existing GNOME extension detection. The screenshot for you will be taken on Omarchy with the Hyprland rule enabled.

## 4. Tickets

**S0. Design tokens and glass shell** (M)
Rewrite `data/base.css`, `dark.css`, `light.css`, `liquid-glass.css` around tokens: radius 30 px, glass fill, edge, shadow, divider, chip, disc-button, text sizes (30 / 17 / 13). Window fully transparent, panel draws the glass. Light mode is the reference look; dark mode is a tinted charcoal glass. Fonts: SF Pro if present, else Inter, else Cantarell.
Done when: empty panel screenshot matches the references' shell (radius, edge, shadow, header height).

**S1. Header and mode model** (M)
Mode glyph + big input + `...` disc. Replace the `All` chip with modes: Spotlight (root), Applications, Files, Actions, Clipboard. Mode shows glyph and placeholder; typing searches inside it. Tab or the buttons switch mode.
Done when: switching mode changes glyph, placeholder and result view.

**S2. Mode buttons row** (S)
Four circular glass buttons (Applications, Files, Actions, Clipboard) shown in the idle state beside a pill search bar, as in the webp. Once a mode is chosen the panel switches to the large header layout. Hidden when typing a query.
Done when: idle state matches the webp composition.

**S3. Applications grid** (L)
Reintroduce grid: category pill row (from `.desktop` Categories mapped to Utilities, Social, Productivity, Photo & Video, Health, Information, Developer, Games, System), 5-column grid, 60 px icons, centered label, top-apps row (frecency top 5) with separator, arrow-key navigation (left/right/up/down), Enter launches, Ctrl+K works on the focused tile.
Done when: matches Spotlight-Applications.jpg; keyboard navigation covers every tile.

**S4. Clipboard rows** (M)
Two-line rows (title, `Text · Copied 4:19 PM` / `URL · host · ...`), content-type icon plus source-app badge (needs the source app at copy time: add optional field to the clipboard store, absent for old items), circular copy button on the right, relative timestamps.
Done when: matches Spotlight-Clipboard.jpg.

**S5. Root results restyle** (M)
Root and query results use the same Tahoe rows: 17 px titles, muted second line, 44 px icons, rounded glass selection pill (low-contrast), section headers in small muted semibold. Remove the footer bar; move the primary-action hint into the selected row's right side (`Open` / `Run`) and keep Ctrl+K reachable via the `...` disc. Toasts become a floating pill under the panel.
Done when: no footer; every previous action still reachable.

**S6. Calculator card, action panel, dialogs** (S)
Restyle to the same glass tokens (card radius, disc buttons, key caps lighter).

**S7. Motion** (S)
Open: fade plus scale 0.96 to 1 in 180 ms. Mode switch: cross-fade 140 ms, panel height animates. Selection pill glides between rows (CSS transition on background only; no layout animation). Respect `gtk-enable-animations`.

**S8. Blur, packaging, verification** (M)
The deployment target is GNOME (Pulsar OS). Blur comes from a GNOME extension (the Pulsar `liquid-glass` one already detected in code, or Blur my Shell); document how to enable it. A Hyprland rule (Lua syntax for Hyprland 0.56) is optional and dev-only, to make previews on Omarchy look closer. This dev machine cannot validate GNOME-only parts (Tracker, extension, gsettings, dock, accent following), so those need a GNOME check. Screenshot script that launches the app, waits for the window, crops to its geometry only (never the full screen), saves root, apps, clipboard, query, Ctrl+K and dark/light shots into `docs/screenshots/`.
Done when: side-by-side with the references reviewed with you.

## 5. Order and size

S0 -> S1 -> S5 (shell, header, rows: most of the visible change) -> S3 -> S4 -> S2 -> S6 -> S7 -> S8.
Rough effort: about 6 to 8 working days. S0+S1+S5 alone already gives a recognizable Tahoe look.

## 6. Decisions needed

1. **Keep the Raycast layout as a second style?** Option: ship `style = tahoe | raycast` in config. Costs extra CSS and layout branches. Default plan: replace, Tahoe only.
2. **Mode shortcuts.** Tahoe uses Cmd+1..4. Here Ctrl+1..9 already picks a result. Default plan: Ctrl+1..9 stays; modes use Alt+1..4.
3. **Footer.** Default plan: remove it (matches Spotlight). Hints move to Ctrl+K and the `...` button.

## 7. Risks

- Without compositor blur the panel can look flat or muddy on busy wallpapers. Mitigation: tuned fill alpha plus gradient, and the blur rule.
- GTK CSS lacks `backdrop-filter`, `overflow` clipping on some versions (already hit once), and `mask`. Rounded clipping may need `Gtk.Overflow::Hidden` set from Rust.
- SF Pro is not redistributable; fallback fonts will differ slightly from the references.
- Source-app badge for clipboard needs focus tracking at copy time; on Wayland this is only reliable through the GNOME extension, so it may be absent.
- Reference images are third-party screenshots; used for visual comparison only and not committed.
