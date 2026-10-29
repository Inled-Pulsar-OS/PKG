# Apple reference notes

Notes on Apple's design behavior that Pulsar Store takes inspiration from. They are written in our own words. We copy ideas and structure, not artwork, icons, fonts, text, or trademarks.

Each claim is marked **Verified** (read in the cited source) or **Unverified** (from memory or secondary sources; check before relying on it). Last checked: 2026-10-28.

## Sources

- [A] [Meet Liquid Glass, WWDC25 session 219](https://developer.apple.com/videos/play/wwdc2025/219/)
- [B] [Mac App Store User Guide, Welcome](https://support.apple.com/guide/app-store/welcome/mac) (the guide does not say which macOS version it describes)
- [C] [Build an AppKit app with the new design, WWDC25 session 310](https://developer.apple.com/videos/play/wwdc2025/310/), known only through search summaries
- The [HIG Materials page](https://developer.apple.com/design/human-interface-guidelines/materials) did not load, so HIG text is not covered here.

## Store structure

| Claim | Status | Source |
|---|---|---|
| The sidebar links to Discover, Arcade, Create, Work, Play, Develop, and Updates | Verified (from the guide's image caption) | [B] |
| Discover offers app recommendations and in-depth stories | Verified | [B] |
| Users can see a summary of an app's privacy practices before getting it | Verified | [B] |
| Users can rate and review apps | Verified | [B] |
| A Categories entry exists in the sidebar | Unverified | |
| Search lives in the toolbar | Unverified | |
| Large editorial cards lead the Discover page | Unverified | |

For Pulsar Store: a sidebar of top-level destinations, an editorial landing page, a privacy and permissions summary on the item page, and an Updates page. Ratings and reviews depend on accounts, so they are out of scope for now.

## Product page and install button

All of this is **Unverified**. It comes from memory and needs a direct look at the App Store on a Mac, or a cited Apple page, before the install button (ticket 5.6) or item page (ticket 7.2) is finalized.

- The page has an icon, a name and subtitle, and a main action button.
- The button moves through states: get, in progress with a progress indicator, then open.
- Below the header sit screenshots, a description, a "what's new" section, and an information grid.

## Liquid Glass

| Claim | Status | Source |
|---|---|---|
| Liquid Glass is a distinct layer for controls and navigation that floats above content | Verified | [A] |
| Its main effect is lensing: it bends and concentrates light instead of scattering it | Verified | [A] |
| It responds to interaction with flexing and an inner glow | Verified | [A] |
| Controls morph into each other as context changes, and glass appears and disappears by changing how it bends light rather than by fading | Verified | [A] |
| It adapts to the content behind it: shadows deepen over text, and small elements can flip between light and dark | Verified | [A] |
| Larger glass looks thicker, with deeper shadows and stronger refraction | Verified | [A] |
| Glass shifts its look and recedes when its window loses focus | Verified | [A] |
| Use glass for the navigation layer only; do not put it on content such as tables | Verified | [A] |
| Do not stack glass on glass; place fills or vibrancy on top of glass instead | Verified | [A] |
| Keep content and glass from intersecting at rest | Verified | [A] |
| Two variants: regular (adaptive, general purpose) and clear (more transparent, needs dimming, only over bold media-rich content). Never mix them | Verified | [A] |
| Tint only primary actions; tinting everything makes nothing stand out | Verified | [A] |
| Scroll edge effects separate content from glass by dissolving content as it passes under | Verified | [A] |
| Glass controls nest into window corners with concentric radii | Verified | [A] |
| Reduced Transparency makes glass frostier, Increased Contrast makes it black or white with a border, Reduced Motion softens effects and removes elastic behavior | Verified | [A] |
| A macOS 27 redesign changes the sidebar and toolbar | Unverified (single blog post, not Apple) | |

## What this means for Pulsar Store

- **Glass surface (ticket 5.3):** navigation layer only, regular variant only, no glass stacking. Provide a frosted flat fallback that also serves Reduced Transparency.
- **Accessibility (tickets 5.3, 8.1, 11.1):** map the three Apple settings to GTK equivalents: reduced transparency to the flat fallback, increased contrast to high-contrast tokens, reduced motion to `gtk-enable-animations`.
- **Motion (ticket 8.x):** prefer morphing and in-place transitions over fades.
- **Radii (ticket 5.4):** controls inside a rounded window use concentric radii.
- **GTK limit:** GTK4 CSS cannot blur what is behind a widget, and lensing needs shader work. Expect a blur, tint, and edge-highlight approximation, not true lensing.
