# 0005 Catalog schema is a public, versioned contract

Status: Accepted

## Context
The catalog is served from three mirrors and consumed by the store, `storectl`, and possibly third parties. Publishers need a stable target.

## Decision
Publish a JSON Schema in `docs/schema/` with a version number. Within a major version, only additive, optional changes are allowed. Text fields may be a string or a map of BCP 47 tag to string. Consumers ignore unknown fields. Fixtures (valid and invalid) live next to the schema and run in tests.

## Consequences
- Older store versions keep working with newer catalogs.
- Breaking changes need a new major version and a migration note.
