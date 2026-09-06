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
            if icon_url and item_id:
                ext = ".png" if not icon_url.endswith(".svg") else ".svg"
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

    def is_installed(self, item: Dict[str, Any]) -> bool:
        itype = item.get("type")
        iid = item.get("id")
        if not iid:
            return False

        if itype == "flatpak":
            return self.flatpak.is_installed(iid)
        elif itype == "system":
            pkg_name = item.get("package_name", iid)
            return self.system.is_installed(pkg_name)
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

        if itype == "flatpak":
            return self.flatpak.get_installed_version(iid)
        elif itype == "system":
            pkg_name = item.get("package_name", iid)
            return self.system.get_installed_version(pkg_name)
        elif itype in ("gnome_extension", "extension"):
            uuid = item.get("metadata", {}).get("uuid") or item.get("uuid", iid)
            return self.gnome_ext.get_installed_version(uuid)
        elif itype == "sayri_skill":
            return self.sayri_skill.get_installed_version(iid)
        elif itype == "sayri_plugin":
            return self.sayri_plugin.get_installed_version(iid)
        return None

    def install(self, item: Dict[str, Any]) -> bool:
        itype = item.get("type")
        iid = item.get("id")
        self.log(f"Initiating installation of {item.get('name', iid)} ({itype})...")

        if itype == "flatpak":
            ref = item.get("metadata", {}).get("flatpakref_url") or item.get("download_url") or iid
            return self.flatpak.install(ref)
        elif itype == "system":
            pkg_name = item.get("package_name", iid)
            deb_url = item.get("deb_url")
            arch_url = item.get("arch_url") or item.get("pacman_url")
            return self.system.install(pkg_name, deb_url=deb_url, arch_url=arch_url)
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

        if itype == "flatpak":
            return self.flatpak.uninstall(iid)
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
