# Publishing to the Pulsar Store catalog

The catalog is a JSON file described by `docs/schema/catalog.v1.schema.json`. Validate yours with:

```
CATALOG_FILE=path/to/index.json cargo test --test schema -- --ignored
```

## An entry

```json
{
  "id": "org.example.App",
  "type": "flatpak",
  "name": { "en": "Example", "es": "Ejemplo" },
  "summary": { "en": "One sentence.", "es": "Una frase." },
  "description": "A longer text. Plain text only.",
  "version": "1.2.0",
  "author": "Example Inc.",
  "category": "Graphics",
  "tags": ["image", "editor"],
  "icon_url": "https://example.org/icon.png",
  "demo_urls": ["https://example.org/shot1.png"],
  "formats": { "flatpak": "https://example.org/app.flatpakref" },
  "github_url": "https://github.com/example/app"
}
```

## Fields

- `id` and `type` are required. Types: `flatpak` (also `app`, `desktop_app`), `system`, `gnome_extension` (also `extension`), `sayri_skill`, `sayri_plugin`.
- `name`, `summary`, `description` can be a string or a map of language tags (`en`, `es`, `es-DO`). The store picks the user's language, then the language without region, then `en`, then the first entry.
- `version` should be dotted numbers (`1.2.0`). The store offers an update only when the catalog version is newer than the installed one.
- `formats` lists native packages (`deb`, `arch`, `pacman`) and `flatpak`. Provide a `sha256` for any download so the store can verify it.
- Editorial: `featured: true`, `story`, and `hero_image` promote an item on Discover.

## Images

Use `https` URLs. Icons: PNG or SVG, up to 2 MB, square. Screenshots: PNG or JPEG, up to 8 MB. The store checks the file type and ignores anything else.

## Rules

- Plain text only. Markup and HTML are shown as typed.
- Only `http` and `https` links are opened.
- Do not include secrets. The catalog is public.
