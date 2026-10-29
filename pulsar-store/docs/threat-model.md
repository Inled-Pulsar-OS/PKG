# Threat model

What can go wrong in Pulsar Store, what already protects against it, and what is still open. Written from the code as of the 2.0 rewrite; each open item is a ticket in `docs/tickets/b11-quality/`.

## What we protect

- The user's system. The store runs package managers as root, so a mistake here is a root compromise.
- The user's files and accounts.
- The user's trust in what the store shows.

## Who we worry about

- **A compromised or malicious catalog mirror**, or someone intercepting its traffic. The catalog decides what gets installed, so this is the main risk.
- **A malicious catalog entry** from a publisher: a bad package, archive, URL, or text.
- **A web page or another local program** that can start the store through a link or D-Bus.

We do not defend against an attacker who already has the user's session or root.

## Surfaces

| Surface | Risk | Today |
|---|---|---|
| Catalog content | A mirror serves entries that install attacker packages | TLS only. No catalog signature (**F2**). Checksums are optional (**F1**). |
| Privileged commands | Argument injection, shell injection | All commands are argv lists, never a shell. Package names starting with `-` or containing spaces are rejected. One escalation path (`core/privilege.rs`). |
| `pacman -U --overwrite "*"` | A package replaces unrelated system files | Open (**F3**). |
| Package downloads | Tampered file, huge file | SHA-256 verified when the catalog declares one (**F1**). Downloads are capped at 4 GiB. HTTPS-to-HTTP redirects are refused. |
| Archives (extensions, skills, plugins) | Zip-slip, zip bombs, bad ids | Entries that escape the folder are rejected. Extraction is capped at 2 GiB and 20,000 entries. Ids and UUIDs must be a single plain path segment. |
| Links and command line | A web page silently installs software | `pulsar://install/<id>` and `--install` open a confirmation dialog before anything installs. Ids are validated. |
| URLs from the catalog | `file:`, `javascript:` or other schemes | Only `http(s)` URLs are fetched or opened. |
| Catalog text and images | Markup injection, decoder bugs | Text is shown as plain text, never as markup. Images are sniffed, size-capped, and decoded by GTK's image loaders. A bug in a decoder is out of our control (**F8**). |
| Update checker D-Bus service | Another session process abuses it | It exposes only "check now" and a count. Checks are rate-limited by the check itself. Accepted. |
| Local state (`installed.json`, `prefs.json`) | Tampering by the same user | Written atomically. A corrupt file is moved aside. Same-user tampering is out of scope. |
| Third-party code (Sayri plugins, GNOME extensions) | Installed code runs with user rights | The store shows the published security report. It cannot sandbox this code. |

## Open findings

| ID | Severity | Finding |
|---|---|---|
| F1 | High | Checksums are optional, so a tampered native package installs as root if the mirror lies. Require a checksum for native packages. |
| F2 | High | The catalog is not signed. Sign it and verify the signature before use. |
| F3 | Medium | `pacman -U --overwrite "*"` lets a package overwrite any file. Narrow or remove it. |
| F6 | Medium | Redirects are checked after the fact (the request is already made). Decide whether to refuse plain HTTP downloads outright. |
| F7 | Done | Link installs were silent. They now ask first. |
| F8 | Low | Image decoders process untrusted files. Consider rendering screenshots only on request. |
| F9 | Low | Install progress parsing trusts command output. Cosmetic only; no action. |

F4 (zip bombs) and F5 (unbounded downloads) were fixed in the same pass as this document.
