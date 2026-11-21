from unittest import TestCase

from tree_sitter import Language, Parser, Query, QueryCursor
import tree_sitter_lp


class TestLanguage(TestCase):
    def test_can_load_grammar(self):
        try:
            Parser(Language(tree_sitter_lp.language()))
        except Exception:
            self.fail("Error loading Lp grammar")


class TestQueries(TestCase):
    NAMES = (
        "HIGHLIGHTS_QUERY",
        "LOCALS_QUERY",
        "FOLDS_QUERY",
        "INDENTS_QUERY",
        "TAGS_QUERY",
    )

    def test_queries_compile(self):
        language = Language(tree_sitter_lp.language())
        for name in self.NAMES:
            with self.subTest(query=name):
                source = getattr(tree_sitter_lp, name)
                self.assertTrue(source.strip())
                Query(language, source)

    def test_tags_find_definitions(self):
        language = Language(tree_sitter_lp.language())
        parser = Parser(language)
        source = b"min\n obj: x\nsubject to\n c1: x >= 1\nend\n"
        tree = parser.parse(source)
        self.assertFalse(tree.root_node.has_error)
        query = Query(language, tree_sitter_lp.TAGS_QUERY)
        captures = QueryCursor(query).captures(tree.root_node)
        self.assertIn("definition.constant", captures)
        self.assertIn("reference.variable", captures)
