# Catalog schema

`catalog.v1.schema.json` is the contract between catalog publishers and the store (ADR 0005).

- Within v1 only additive, optional changes are allowed. Consumers ignore unknown fields.
- The top level needs `packages` (or the legacy `items`). Each item needs `id` and `type`.
- `name`, `summary`, and `description` are a string or a map of BCP 47 tag to string.
  The store picks text in this order: exact tag (`es-DO`), language (`es`), `en`, first entry.
- Known `type` values: `flatpak` (aliases `app`, `desktop_app`), `system`, `gnome_extension` (alias `extension`), `sayri_skill`, `sayri_plugin`.

Fixtures in `fixtures/valid` must validate and parse; fixtures in `fixtures/invalid` must be rejected. Both run in `tests/schema.rs`.

To check a real catalog: `CATALOG_FILE=path/to/index.json cargo test --test schema -- --ignored`.
