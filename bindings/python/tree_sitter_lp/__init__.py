"""Tree-sitter grammar for LP (Linear Programming) files.

Use :func:`language` with ``tree_sitter.Language`` to build a parser.
"""

from importlib.resources import files as _files
from pathlib import Path as _Path

from ._binding import language

_QUERIES = {
    "HIGHLIGHTS_QUERY": "highlights.scm",
    "LOCALS_QUERY": "locals.scm",
    "FOLDS_QUERY": "folds.scm",
    "INDENTS_QUERY": "indents.scm",
    "TAGS_QUERY": "tags.scm",
}


def __getattr__(name):
    # Read lazily. Built wheels bundle the queries; an editable install reads
    # them from the repository checkout instead.
    if name in _QUERIES:
        bundled = _files(__package__) / "queries" / _QUERIES[name]
        if bundled.is_file():
            query = bundled.read_text()
        else:
            checkout = _Path(__file__).resolve().parents[3] / "queries" / _QUERIES[name]
            query = checkout.read_text()
        globals()[name] = query
        return query
    raise AttributeError(f"module {__name__!r} has no attribute {name!r}")

__all__ = [
    "language",
    "HIGHLIGHTS_QUERY",
    "LOCALS_QUERY",
    "FOLDS_QUERY",
    "INDENTS_QUERY",
    "TAGS_QUERY",
]
