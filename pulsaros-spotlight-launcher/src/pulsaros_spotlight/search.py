"""Tracker/TinySPARQL search backend for PulsarOS Spotlight."""

from __future__ import annotations

import logging
from dataclasses import dataclass
from typing import TYPE_CHECKING

from pulsaros_spotlight.apps import DesktopApp, load_apps

if TYPE_CHECKING:
    from pulsaros_spotlight.clipboard import ClipboardManager

logger = logging.getLogger(__name__)

_BUS_NAME = "org.freedesktop.LocalSearch3"

# -- SPARQL query templates ---------------------------------------------------

_SEARCH_ALL = """\
SELECT ?url ?title ?mtime WHERE {{
    ?u a nfo:FileDataObject ;
       nie:url ?url ;
       nfo:fileName ?title ;
       nfo:fileLastModified ?mtime .
    FILTER(CONTAINS(LCASE(?title), LCASE("{query}")))
}} ORDER BY ?title LIMIT {limit}
"""

_SEARCH_APPS = """\
SELECT ?url ?name WHERE {{
    ?u a nfo:Software ;
       nie:url ?url ;
       nie:title ?name .
    FILTER(CONTAINS(LCASE(?name), LCASE("{query}")))
}} ORDER BY ?name LIMIT {limit}
"""

_SEARCH_CATEGORY = """\
SELECT ?url ?title ?mtime WHERE {{
    ?u a {rdf_type} ;
       nie:url ?url ;
       nfo:fileName ?title ;
       nfo:fileLastModified ?mtime .
    FILTER(CONTAINS(LCASE(?title), LCASE("{query}")))
}} ORDER BY ?title LIMIT {limit}
"""

_CATEGORY_RDF_TYPES: dict[str, str] = {
    "documents": "nie:InformationElement",
    "images": "nfo:Image",
    "audio": "nmm:MusicPiece",
    "video": "nmm:Video",
}

DEFAULT_LIMIT = 50


@dataclass(frozen=True)
class SearchResult:
    """Single search result from Tracker or internal providers."""

    url: str
    title: str
    mime: str = ""
    snippet: str = ""
    app: DesktopApp | None = None


class SearchBackend:
    """SPARQL query layer over Tracker/TinySPARQL and local providers."""

    def __init__(self, clipboard_mgr: ClipboardManager | None = None) -> None:
        self._conn = None
        self._clipboard_mgr = clipboard_mgr
        self._apps: list[DesktopApp] = []
        self._load_apps()

    def connect(self) -> bool:
        """Connect to the Tracker SPARQL endpoint over D-Bus."""
        try:
            from gi.repository import Tsparql

            self._conn = Tsparql.SparqlConnection.bus_new(_BUS_NAME, None, None)
            logger.info("Connected to %s", _BUS_NAME)
            return True
        except Exception:
            logger.warning("Failed to connect to Tracker (%s)", _BUS_NAME)
            self._conn = None
            return False

    def _load_apps(self) -> None:
        """Load desktop applications for app search."""
        try:
            self._apps = load_apps()
        except Exception:
            logger.warning("Failed to load desktop applications", exc_info=True)
            self._apps = []

    @property
    def is_ready(self) -> bool:
        return self._conn is not None

    def search(
        self,
        query: str,
        category: str = "all",
        limit: int = DEFAULT_LIMIT,
    ) -> list[SearchResult]:
        """Run a full-text search and return results."""
        if category == "clipboard":
            if self._clipboard_mgr:
                return self._clipboard_mgr.search_history(query)
            return []

        if not query.strip():
            return []

        query = query.strip()

        if category in ("apps", "applications"):
            return self._search_apps(query, limit)

        if category == "all":
            app_results = self._search_apps(query, limit)
            clip_results = []
            if self._clipboard_mgr:
                clip_results = self._clipboard_mgr.search_history(query)[:3]

            if self.is_ready:
                sparql = self._build_query(query, category, limit)
                file_results = self._execute(sparql)
                return clip_results + app_results + file_results
            return clip_results + app_results

        if not self.is_ready:
            return []

        sparql = self._build_query(query, category, limit)
        return self._execute(sparql)

    def _search_apps(self, query: str, limit: int) -> list[SearchResult]:
        """Search desktop applications by name and comment."""
        query_lower = query.lower()
        results: list[SearchResult] = []

        for app in self._apps:
            if query_lower in app.name.lower() or (app.comment and query_lower in app.comment.lower()):
                results.append(
                    SearchResult(
                        url=f"app://{app.id}",
                        title=app.name,
                        mime="application/x-desktop",
                        snippet=app.comment or "",
                        app=app,
                    )
                )
                if len(results) >= limit:
                    break

        return results

    # -- internal -------------------------------------------------------------

    def _build_query(self, query: str, category: str, limit: int) -> str:
        if category in ("apps", "applications"):
            return _SEARCH_APPS.format(query=query, limit=limit)
        if category in _CATEGORY_RDF_TYPES:
            return _SEARCH_CATEGORY.format(
                query=query, rdf_type=_CATEGORY_RDF_TYPES[category], limit=limit
            )
        return _SEARCH_ALL.format(query=query, limit=limit)

    def _execute(self, sparql: str) -> list[SearchResult]:
        try:
            cursor = self._conn.query(sparql, None)
        except Exception:
            logger.warning("SPARQL query failed", exc_info=True)
            return []

        results: list[SearchResult] = []
        while cursor.next():
            url = cursor.get_string(0) or ""
            title = cursor.get_string(1) or url.rsplit("/", 1)[-1]
            results.append(
                SearchResult(url=url, title=title, mime="", snippet="", app=None)
            )
        return results
