"""Tree-sitter grammar for LP (Linear Programming) files.

Use :func:`language` with ``tree_sitter.Language`` to build a parser.
"""

from importlib.resources import files as _files

from ._binding import language

HIGHLIGHTS_QUERY = (_files(__package__) / "queries/highlights.scm").read_text()
LOCALS_QUERY = (_files(__package__) / "queries/locals.scm").read_text()

__all__ = ["language", "HIGHLIGHTS_QUERY", "LOCALS_QUERY"]
