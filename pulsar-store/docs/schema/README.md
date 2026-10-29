# Catalog schema

`catalog.v1.schema.json` is the contract between catalog publishers and the store (ADR 0005).

- Within v1 only additive, optional changes are allowed. Consumers ignore unknown fields.
- The top level needs `packages` (or the legacy `items`). Each item needs `id` and `type`.
- `name`, `summary`, and `description` are a string or a map of BCP 47 tag to string.
  The store picks text in this order: exact tag (`es-DO`), language (`es`), `en`, first entry.
- Known `type` values: `flatpak` (aliases `app`, `desktop_app`), `system`, `gnome_extension` (alias `extension`), `sayri_skill`, `sayri_plugin`.

Fixtures in `fixtures/valid` must validate and parse; fixtures in `fixtures/invalid` must be rejected. Both run in `tests/schema.rs`.

To check a real catalog: `CATALOG_FILE=path/to/index.json cargo test --test schema -- --ignored`.

## Editorial fields (optional, added within v1)

- `featured` (boolean): show the item on Discover. Featured items lead the page.
- `tag` (localized text): the small label above a card or banner title, such as "Quick look" or "Life hack". When absent, the item's type is shown.
- `story` (localized text): short text shown on the hero card instead of the summary.
- `hero_image` (URL): wide image behind the hero card.
- `demo_urls` (array of URLs) and `promo_url` (URL): screenshots on the item page.

Older stores ignore these fields, and older catalogs without them still work.
