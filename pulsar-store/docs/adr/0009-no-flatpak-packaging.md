# 0009 Do not package the store as a Flatpak

Status: Accepted

## Context
Ticket 10.3 asked whether to ship Pulsar Store as a Flatpak. The store's job is to run `pacman`, `apt-get`, `pkexec`, and manage GNOME extensions on the host.

## Decision
Ship as native packages (deb and Arch) only. Do not publish a Flatpak.

## Consequences
- A Flatpak sandbox cannot run host package managers or write to the host's extension folders. The only way around it is `flatpak-spawn --host` or broad filesystem permissions, which removes the sandbox and adds a risky escape path.
- Users on systems without native packages for Pulsar OS are out of scope; Pulsar OS ships the native packages.
- Revisit if a Flatpak-only edition (managing only Flatpak apps) becomes useful.
