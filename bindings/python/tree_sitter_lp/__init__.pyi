from typing import Final
from typing_extensions import CapsuleType

HIGHLIGHTS_QUERY: Final[str]
"""The syntax highlighting query for this grammar."""

LOCALS_QUERY: Final[str]
"""The local variable query for this grammar."""

FOLDS_QUERY: Final[str]
"""The code folding query for this grammar."""

INDENTS_QUERY: Final[str]
"""The indentation query for this grammar."""

TAGS_QUERY: Final[str]
"""The symbol tagging query for this grammar."""

def language() -> CapsuleType:
    """Return the LP language as a capsule for ``tree_sitter.Language``.

Example::

    from tree_sitter import Language, Parser
    import tree_sitter_lp

    parser = Parser(Language(tree_sitter_lp.language()))
"""
