"""Tree-sitter grammar for LP (Linear Programming) files.

Use :func:`language` with ``tree_sitter.Language`` to build a parser.
"""

from importlib.resources import files as _files

from ._binding import language

_QUERIES = {"HIGHLIGHTS_QUERY": "highlights.scm", "LOCALS_QUERY": "locals.scm"}


def __getattr__(name):
    # Read lazily: queries are only bundled into built wheels, not editable installs.
    if name in _QUERIES:
        query = (_files(__package__) / "queries" / _QUERIES[name]).read_text()
        globals()[name] = query
        return query
    raise AttributeError(f"module {__name__!r} has no attribute {name!r}")

__all__ = ["language", "HIGHLIGHTS_QUERY", "LOCALS_QUERY"]
