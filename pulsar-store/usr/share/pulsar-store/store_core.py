#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Core Store Manager for Pulsar Store."""

import os
import json
import time
import shutil
import urllib.request
import threading
from typing import Dict, Any, List, Optional, Callable

from backends import (
    FlatpakBackend,
    SystemBackend,
    GnomeExtensionBackend,
    SayriSkillBackend,
    SayriPluginBackend,
)


CATALOG_URLS = [
    "https://store-os.inled.es/schema/index.json",
    "https://raw.githubusercontent.com/Inled-Pulsar-OS/store/main/schema/index.json",
    "https://pulsar-store.pages.dev/schema/index.json",
]


class StoreCore:
    def __init__(self, log_fn: Optional[Callable[[str], None]] = None, icon_loaded_cb: Optional[Callable[[], None]] = None):
        self.log = log_fn or (lambda msg: None)
        self.icon_loaded_cb = icon_loaded_cb or (lambda: None)
        self.cache_dir = os.path.expanduser("~/.cache/pulsar-store")
        self.icons_dir = os.path.join(self.cache_dir, "icons")
        os.makedirs(self.icons_dir, exist_ok=True)
        self.catalog_cache_path = os.path.join(self.cache_dir, "catalog.json")

        self.flatpak = FlatpakBackend(self.log)
        self.system = SystemBackend(self.log)
        self.gnome_ext = GnomeExtensionBackend(self.log)
        self.sayri_skill = SayriSkillBackend(self.log)
        self.sayri_plugin = SayriPluginBackend(self.log)

        self.catalog: Dict[str, Any] = {"packages": []}
        self.items: List[Dict[str, Any]] = []
        self.last_error: Optional[str] = None
        self.load_catalog()

    def _normalize_items(self, raw_data: Any) -> List[Dict[str, Any]]:
        if not isinstance(raw_data, dict):
            return []
        pkgs = raw_data.get("packages") or raw_data.get("items") or []
        normalized = []
        for p in pkgs:
            if not isinstance(p, dict):
                continue
            item = dict(p)
            if not item.get("summary") and item.get("description"):
                item["summary"] = item["description"].split(". ")[0] + "."
            normalized.append(item)
        return normalized

    def load_catalog(self):
        """Loads cached catalog or bundled default catalog."""
        local_path = os.path.join(os.path.dirname(os.path.abspath(__file__)), "catalog.json")
        bundled_path = "/usr/share/pulsar-store/catalog.json"

        if os.path.isfile(self.catalog_cache_path):
            try:
                with open(self.catalog_cache_path, "r", encoding="utf-8") as f:
                    self.catalog = json.load(f)
                    self.items = self._normalize_items(self.catalog)
                    threading.Thread(target=self._prefetch_icons, daemon=True).start()
                    return
            except Exception as e:
                self.log(f"Failed to load cache catalog: {e}")

        for p in (local_path, bundled_path):
            if os.path.isfile(p):
                try:
                    with open(p, "r", encoding="utf-8") as f:
                        self.catalog = json.load(f)
                        self.items = self._normalize_items(self.catalog)
                        threading.Thread(target=self._prefetch_icons, daemon=True).start()
                        return
                except Exception as e:
                    self.log(f"Failed to load catalog from {p}: {e}")

        self.items = []

    def refresh_catalog(self, force: bool = False) -> bool:
        """Fetches remote catalog from official Pulsar Store repository."""
        for url in CATALOG_URLS:
            try:
                self.log(f"Fetching catalog from {url}...")
                req = urllib.request.Request(url, headers={"User-Agent": "PulsarStore/1.0"})
                with urllib.request.urlopen(req, timeout=6) as resp:
                    if resp.status == 200:
                        data = json.loads(resp.read().decode("utf-8"))
                        if isinstance(data, dict):
                            self.catalog = data
                            self.items = self._normalize_items(data)
                            with open(self.catalog_cache_path, "w", encoding="utf-8") as f:
                                json.dump(self.catalog, f, indent=2)
                            self.log(f"Catalog updated from {url} with {len(self.items)} packages.")
                            threading.Thread(target=self._prefetch_icons, daemon=True).start()
                            return True
            except Exception as e:
                self.log(f"Catalog fetch from {url} failed: {e}")

        self.load_catalog()
        return False

    def _prefetch_icons(self):
        new_downloads = False
        for item in self.items:
            icon_url = item.get("icon_url")
            item_id = item.get("id")
            if not item_id:
                continue

            if not icon_url:
                icon_url = f"https://raw.githubusercontent.com/Inled-Pulsar-OS/store/main/assets/icons/{item_id}.png"
            elif not (icon_url.startswith("http://") or icon_url.startswith("https://")):
                clean_path = icon_url.lstrip("/")
                icon_url = f"https://raw.githubusercontent.com/Inled-Pulsar-OS/store/main/{clean_path}"

            ext = ".svg" if icon_url.endswith(".svg") else ".png"
            icon_file = os.path.join(self.icons_dir, f"{item_id}{ext}")
            if not os.path.isfile(icon_file):
                try:
                    req = urllib.request.Request(icon_url, headers={"User-Agent": "PulsarStore/1.0"})
                    with urllib.request.urlopen(req, timeout=10) as resp, open(icon_file, "wb") as f:
                        shutil.copyfileobj(resp, f)
                    new_downloads = True
                except Exception as e:
                    self.log(f"Failed to download icon for {item_id}: {e}")
        if new_downloads and self.icon_loaded_cb:
            self.icon_loaded_cb()

    def get_cached_icon(self, item: Dict[str, Any]) -> Optional[str]:
        item_id = item.get("id")
        if item_id:
            for ext in (".png", ".svg", ".jpg", ".jpeg"):
                p = os.path.join(self.icons_dir, f"{item_id}{ext}")
                if os.path.isfile(p):
                    return p
            # Check local bundled icons
            for base_dir in ["/usr/share/pulsar-store/assets/icons", os.path.join(os.path.dirname(__file__), "../assets/icons")]:
                for ext in (".png", ".svg"):
                    local_p = os.path.join(base_dir, f"{item_id}{ext}")
                    if os.path.isfile(local_p):
                        return local_p
        return None

    def get_announcement(self) -> Optional[Dict[str, Any]]:
        return self.catalog.get("announcement")

    def get_items_by_category(self, category: str) -> List[Dict[str, Any]]:
        category = category.lower()
        if category in ("all", "discover"):
            return self.items
        elif category == "apps":
            return [i for i in self.items if i.get("type") in ("flatpak", "system", "app")]
        elif category == "extensions":
            return [i for i in self.items if i.get("type") in ("gnome_extension", "extension")]
        elif category == "sayri":
            return [i for i in self.items if i.get("type") in ("sayri_skill", "sayri_plugin")]
        elif category == "installed":
            return [i for i in self.items if self.is_installed(i)]
        else:
            return [i for i in self.items if i.get("category", "").lower() == category]

    def search(self, query: str, category: Optional[str] = None) -> List[Dict[str, Any]]:
        query = query.strip().lower()
        items = self.get_items_by_category(category) if category else self.items
        if not query:
            return items

        results = []
        for item in items:
            name = item.get("name", "").lower()
            summary = item.get("summary", "").lower()
            desc = item.get("description", "").lower()
            item_id = item.get("id", "").lower()
            tags = " ".join(item.get("tags", [])).lower()

            if (query in name or query in summary or query in desc or
                    query in item_id or query in tags):
                results.append(item)
        return results

    def _ledger_path(self) -> str:
        return os.path.join(self.cache_dir, "installed.json")

    def _read_ledger(self) -> Dict[str, Any]:
        try:
            with open(self._ledger_path(), "r", encoding="utf-8") as f:
                return json.load(f)
        except Exception:
            return {}

    def _write_ledger(self, data: Dict[str, Any]):
        try:
            with open(self._ledger_path(), "w", encoding="utf-8") as f:
                json.dump(data, f, indent=2)
        except Exception as e:
            self.log(f"Failed to write install ledger: {e}")

    def _record_install(self, iid: str, edition: str, pkg_name: Optional[str], version: Optional[str]):
        if not iid:
            return
        data = self._read_ledger()
        entry: Dict[str, Any] = {"edition": edition}
        if pkg_name:
            entry["package_name"] = pkg_name
        if version:
            entry["version"] = version
        data[iid] = entry
        self._write_ledger(data)

    def _forget_install(self, iid: str):
        data = self._read_ledger()
        if iid in data:
            del data[iid]
            self._write_ledger(data)

    def _native_candidate_names(self, item: Dict[str, Any]) -> List[str]:
        """English: Candidate native package names for an item, from metadata and
        edition URLs. / Español: Nombres de paquete nativo candidatos de un ítem."""
        names: List[str] = []
        iid = item.get("id")
        pkg = item.get("package_name")
        if pkg:
            names.append(pkg)
        if iid:
            names.append(iid)
        for e in self.get_editions(item):
            if e["key"] in ("arch", "debian"):
                bn = e.get("url", "").split("?")[0].split("/")[-1]
                for suffix in (".pkg.tar.zst", ".pkg.tar.xz", ".pkg.tar", ".deb"):
                    if bn.endswith(suffix):
                        bn = bn[: -len(suffix)]
                if bn:
                    names.append(bn)
        seen: List[str] = []
        for n in names:
            if n and n not in seen:
                seen.append(n)
        return seen

    def _find_installed_native(self, item: Dict[str, Any]) -> Optional[tuple]:
        """English: Detects a native (pacman/apt) installation of an item, using
        explicit names first and then a name/id substring scan.
        Español: Detecta una instalación nativa de un ítem por nombres explícitos
        o coincidencia de cadenas."""
        for name in self._native_candidate_names(item):
            if self.system.is_installed(name):
                return (name, "arch" if self.system.is_arch else "debian")
        iid = item.get("id", "")
        if not iid:
            return None
        installed = self.system.list_installed_names()
        for pkg in installed:
            if len(pkg) < 4:
                continue
            if pkg == iid or iid in pkg or pkg in iid:
                return (pkg, "arch" if self.system.is_arch else "debian")
        return None

    def get_installed_edition(self, item: Dict[str, Any]) -> Optional[str]:
        """English: Returns the edition in which an item is currently installed
        ('arch'/'debian'/'flatpak') or None.
        Español: Devuelve la edición en que un ítem está instalado o None."""
        itype = item.get("type")
        iid = item.get("id")
        if not iid:
            return None

        if itype in ("flatpak", "app", "desktop_app"):
            ledger = self._read_ledger().get(iid)
            if ledger:
                return ledger.get("edition", "flatpak")
            native = self._find_installed_native(item)
            if native:
                return native[1]
            if self.flatpak.is_installed(iid, item=item):
                return "flatpak"
            return None
        elif itype == "system":
            if self.system.is_installed(item.get("package_name", iid)):
                return "arch" if self.system.is_arch else "debian"
            if self.flatpak.is_installed(iid, item=item):
                return "flatpak"
            return None
        elif itype in ("gnome_extension", "extension"):
            uuid = item.get("metadata", {}).get("uuid") or item.get("uuid", iid)
            return "extension" if self.gnome_ext.is_installed(uuid) else None
        elif itype == "sayri_skill":
            return "skill" if self.sayri_skill.is_installed(iid) else None
        elif itype == "sayri_plugin":
            return "plugin" if self.sayri_plugin.is_installed(iid) else None
        return None

    def is_installed(self, item: Dict[str, Any]) -> bool:
        itype = item.get("type")
        iid = item.get("id")
        if not iid:
            return False

        if itype in ("flatpak", "app", "desktop_app"):
            return self.get_installed_edition(item) is not None
        elif itype == "system":
            return self.get_installed_edition(item) is not None
        elif itype in ("gnome_extension", "extension"):
            uuid = item.get("metadata", {}).get("uuid") or item.get("uuid", iid)
            return self.gnome_ext.is_installed(uuid)
        elif itype == "sayri_skill":
            return self.sayri_skill.is_installed(iid)
        elif itype == "sayri_plugin":
            return self.sayri_plugin.is_installed(iid)
        return False

    def get_installed_version(self, item: Dict[str, Any]) -> Optional[str]:
        itype = item.get("type")
        iid = item.get("id")
        if not iid:
            return None

        if itype in ("flatpak", "app", "desktop_app"):
            ledger = self._read_ledger().get(iid) or {}
            edition = ledger.get("edition") or self.get_installed_edition(item)
            pkg_name = ledger.get("package_name")
            if edition in ("arch", "debian"):
                if pkg_name:
                    ver = self.system.get_installed_version(pkg_name)
                    if ver:
                        return ver
                native = self._find_installed_native(item)
                if native:
                    ver = self.system.get_installed_version(native[0])
                    if ver:
                        return ver
            elif edition == "flatpak":
                ver = self.flatpak.get_installed_version(iid, item=item)
                if ver:
                    return ver
            return ledger.get("version")
        elif itype == "system":
            native = self._find_installed_native(item)
            if native:
                ver = self.system.get_installed_version(native[0])
                if ver:
                    return ver
            return self.flatpak.get_installed_version(iid, item=item)
        elif itype in ("gnome_extension", "extension"):
            uuid = item.get("metadata", {}).get("uuid") or item.get("uuid", iid)
            return self.gnome_ext.get_installed_version(uuid)
        elif itype == "sayri_skill":
            return self.sayri_skill.get_installed_version(iid)
        elif itype == "sayri_plugin":
            return self.sayri_plugin.get_installed_version(iid)
        return None

    def get_editions(self, item: Dict[str, Any]) -> List[Dict[str, Any]]:
        """English: Lists the installable editions of an item, mirroring the web
        store. Native (Debian/Arch) editions are shown whenever the catalog
        provides them; the Flatpak edition is offered when explicitly present in
        'formats', referenced in metadata, or when it is the only available option.
        Español: Lista las ediciones instalables de un ítem, igual que la web.
        Las ediciones nativas (Debian/Arch) se muestran cuando el catálogo las
        tiene; la edición Flatpak se ofrece si está explícita en 'formats', en
        metadata, o cuando es la única opción disponible."""
        f = item.get("formats", {}) or {}
        meta = item.get("metadata", {}) or {}
        url = item.get("download_url", "") or ""
        editions: List[Dict[str, Any]] = []

        if f.get("deb") or url.endswith(".deb"):
            editions.append({
                "key": "debian",
                "label": "Debian Edition",
                "desc": "Native .deb package (Pulsar OS Debian, Ubuntu)",
                "url": f.get("deb") or (url if url.endswith(".deb") else ""),
            })

        if f.get("arch") or f.get("pacman") or url.endswith((".pkg.tar.zst", ".pkg.tar.xz", ".pacman")):
            arch_url = f.get("arch") or f.get("pacman")
            if not arch_url and url.endswith((".pkg.tar.zst", ".pkg.tar.xz", ".pacman")):
                arch_url = url
            editions.append({
                "key": "arch",
                "label": "Arch Edition",
                "desc": "Native .pkg.tar.zst package (Pulsar OS Arch, Manjaro)",
                "url": arch_url or "",
            })

        if f.get("flatpak") or meta.get("flatpakref_url") or url.endswith((".flatpak", ".flatpakref")) or not editions:
            flatpak_url = f.get("flatpak") or meta.get("flatpakref_url") or (url if url.endswith((".flatpak", ".flatpakref")) else "")
            editions.append({
                "key": "flatpak",
                "label": "Flatpak",
                "desc": "Universal sandboxed container",
                "url": flatpak_url or "",
            })
        return editions

    def best_edition(self, item: Dict[str, Any]) -> Optional[Dict[str, Any]]:
        """English: Returns the edition that works on the current host, preferring
        a native package (Arch -> pacman, Debian -> apt) over Flatpak.
        Español: Devuelve la edición que funciona en este host, prefiriendo el
        paquete nativo (Arch -> pacman, Debian -> apt) sobre Flatpak."""
        editions = self.get_editions(item)
        if not editions:
            return None
        if self.system.is_arch:
            return next((e for e in editions if e["key"] == "arch"), editions[0])
        if self.system.is_debian:
            return next((e for e in editions if e["key"] == "debian"), editions[0])
        return next((e for e in editions if e["key"] == "flatpak"), editions[0])

    def install(self, item: Dict[str, Any], edition: Optional[str] = None) -> bool:
        itype = item.get("type")
        iid = item.get("id")
        self.last_error = None
        self.log(f"Initiating installation of {item.get('name', iid)} ({itype})...")

        if itype in ("flatpak", "app", "desktop_app"):
            editions = {e["key"]: e for e in self.get_editions(item)}
            chosen = edition if (edition and edition in editions) else None
            if not chosen:
                best = self.best_edition(item)
                chosen = best["key"] if best else None

            self.log(f"[Store] Installing '{item.get('name', iid)}' using edition: {chosen or 'none'}")

            if chosen == "arch":
                url = editions.get("arch", {}).get("url")
                if not url:
                    self.last_error = "This package does not provide an Arch Linux edition."
                    return False
                ok = self.system.install_from_url(url)
                self.last_error = self.system.last_error
                if ok:
                    pkg_name = self.system.last_installed_name
                    version = self.system.get_installed_version(pkg_name) if pkg_name else None
                    self._record_install(iid, "arch", pkg_name, version)
                return ok
            elif chosen == "debian":
                url = editions.get("debian", {}).get("url")
                if not url:
                    self.last_error = "This package does not provide a Debian edition."
                    return False
                ok = self.system.install_from_url(url)
                self.last_error = self.system.last_error
                if ok:
                    pkg_name = self.system.last_installed_name
                    version = self.system.get_installed_version(pkg_name) if pkg_name else None
                    self._record_install(iid, "debian", pkg_name, version)
                return ok
            elif chosen == "flatpak":
                flatpak_url = editions.get("flatpak", {}).get("url") or iid
                if self.flatpak.is_available():
                    ok = self.flatpak.install(iid, flatpak_ref=flatpak_url, item=item)
                    self.last_error = getattr(self.flatpak, "last_error", None)
                    if ok:
                        self._record_install(iid, "flatpak", None, self.flatpak.get_installed_version(iid, item=item))
                    return ok
                self.last_error = "The selected Flatpak edition requires the Flatpak CLI, which is not installed."
                self.log("[Store] Flatpak edition selected but 'flatpak' is not available.")
                return False
            return False
        elif itype == "system":
            pkg_name = item.get("package_name", iid)
            deb_url = item.get("deb_url")
            arch_url = item.get("arch_url") or item.get("pacman_url")
            ok = self.system.install(pkg_name, deb_url=deb_url, arch_url=arch_url)
            self.last_error = self.system.last_error
            return ok
        elif itype in ("gnome_extension", "extension"):
            uuid = item.get("metadata", {}).get("uuid") or item.get("uuid", iid)
            url = item.get("download_url", "")
            return self.gnome_ext.install(uuid, url)
        elif itype == "sayri_skill":
            url = item.get("download_url", "")
            raw = item.get("raw_content")
            return self.sayri_skill.install(iid, download_url=url, raw_content=raw)
        elif itype == "sayri_plugin":
            url = item.get("download_url", "")
            return self.sayri_plugin.install(iid, download_url=url)

        return False

    def uninstall(self, item: Dict[str, Any]) -> bool:
        itype = item.get("type")
        iid = item.get("id")
        self.log(f"Initiating uninstallation of {item.get('name', iid)} ({itype})...")

        if itype in ("flatpak", "app", "desktop_app"):
            ledger = self._read_ledger().get(iid) or {}
            edition = ledger.get("edition") or self.get_installed_edition(item)
            if edition in ("arch", "debian"):
                pkg_name = ledger.get("package_name")
                if not pkg_name:
                    native = self._find_installed_native(item)
                    pkg_name = native[0] if native else None
                if not pkg_name:
                    self.last_error = "Could not determine the native package name to remove."
                    return False
                ok = self.system.uninstall(pkg_name)
                if ok:
                    self._forget_install(iid)
                return ok
            ok = self.flatpak.uninstall(iid, item=item)
            if ok:
                self._forget_install(iid)
            return ok
        elif itype == "system":
            pkg_name = item.get("package_name", iid)
            return self.system.uninstall(pkg_name)
        elif itype in ("gnome_extension", "extension"):
            uuid = item.get("metadata", {}).get("uuid") or item.get("uuid", iid)
            return self.gnome_ext.uninstall(uuid)
        elif itype == "sayri_skill":
            return self.sayri_skill.uninstall(iid)
        elif itype == "sayri_plugin":
            return self.sayri_plugin.uninstall(iid)

        return False

    def check_updates(self) -> List[Dict[str, Any]]:
        """Returns list of items that have available updates."""
        updates = []
        for item in self.items:
            if not self.is_installed(item):
                continue
            cur_ver = self.get_installed_version(item)
            new_ver = item.get("version")
            if cur_ver and new_ver and cur_ver != "installed":
                if cur_ver != new_ver:
                    updates.append({
                        "item": item,
                        "current_version": cur_ver,
                        "available_version": new_ver,
                    })
        return updates

    def update_item(self, item: Dict[str, Any]) -> bool:
        return self.install(item)
