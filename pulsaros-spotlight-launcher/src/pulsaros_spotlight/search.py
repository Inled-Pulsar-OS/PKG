"""Tracker/TinySPARQL search backend for PulsarOS Spotlight."""

from __future__ import annotations

import logging
from dataclasses import dataclass

logger = logging.getLogger(__name__)

_BUS_NAME = "org.freedesktop.LocalSearch3"

# -- SPARQL query templates ---------------------------------------------------

_SEARCH_ALL = """\
SELECT ?url ?title ?mime ?mtime (fts:snippet(?u) AS ?snippet) WHERE {{
    ?u a nfo:FileDataObject ;
       nie:url ?url ;
       nie:title ?title ;
       nfo:mimeType ?mime ;
       nfo:contentAccessed ?mtime ;
       fts:match "{query}" .
}} ORDER BY ASC(fts:rank(?u)) LIMIT {limit}
"""

_SEARCH_APPS = """\
SELECT ?url ?name WHERE {{
    ?u a nfo:Software ;
       nie:url ?url ;
       nie:title ?name ;
       fts:match "{query}" .
}} ORDER BY ASC(fts:rank(?u)) LIMIT {limit}
"""

_SEARCH_CATEGORY = """\
SELECT ?url ?title ?mime ?mtime (fts:snippet(?u) AS ?snippet) WHERE {{
    ?u a {rdf_type} ;
       nie:url ?url ;
       nie:title ?title ;
       nfo:mimeType ?mime ;
       nfo:contentAccessed ?mtime ;
       fts:match "{query}" .
}} ORDER BY ASC(fts:rank(?u)) LIMIT {limit}
"""

_CATEGORY_RDF_TYPES: dict[str, str] = {
    "documents": "nie:InformationElement",
    "images": "nfo:Image",
    "music": "nmm:MusicPiece",
    "video": "nmm:Video",
}

DEFAULT_LIMIT = 50


@dataclass(frozen=True)
class SearchResult:
    """Single search result from Tracker."""

    url: str
    title: str
    mime: str
    snippet: str


class SearchBackend:
    """SPARQL query layer over Tracker/TinySPARQL."""

    def __init__(self) -> None:
        self._conn = None

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
        if not self.is_ready or not query.strip():
            return []

        query = query.strip()
        sparql = self._build_query(query, category, limit)
        return self._execute(sparql)

    # -- internal -------------------------------------------------------------

    def _build_query(self, query: str, category: str, limit: int) -> str:
        if category == "apps":
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
            n_cols = cursor.get_n_columns()
            url = cursor.get_string(0) or ""
            title = cursor.get_string(1) or url.rsplit("/", 1)[-1]
            mime = cursor.get_string(2) if n_cols > 2 else ""
            snippet = cursor.get_string(4) if n_cols > 4 else ""
            results.append(
                SearchResult(url=url, title=title, mime=mime, snippet=snippet)
            )
        return results
