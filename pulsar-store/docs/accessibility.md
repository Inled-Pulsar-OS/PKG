# Accessibility

## What is built in

- Every interactive widget has an accessible name. Install buttons read as "Get: Firefox", so a screen reader knows which item they act on. Icon-only buttons have a tooltip and label.
- Tiles and cards are focusable. Enter or Space opens the item; the keyboard works everywhere without a mouse.
- Headings (page titles, section titles) have the heading role.
- Decorative icons are marked presentational.
- Animations stop when the system turns animations off. Glass becomes a flat surface in high-contrast mode.
- Text colors derive from libadwaita's palette, so they follow the system light, dark, and high-contrast settings. Pills mix the accent with the text color so they stay readable in both themes.

## Keyboard

| Keys | Action |
|---|---|
| Ctrl+F | Focus search |
| Ctrl+R | Refresh the catalog |
| Alt+Left | Back |
| Left / Right | Move between screenshots |
| Enter / Space | Open the focused item |
| Esc (in search) | Clear search |

## Not yet verified

These need a person with the tools, and are tracked in `docs/tickets/b11-quality/11.1-accessibility-pass.md`:

- A walkthrough with Orca on every page.
- A contrast measurement of each text color in light, dark, and the flat glass fallback.
- Focus order on the item page with all sections shown.
