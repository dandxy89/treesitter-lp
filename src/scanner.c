// External scanner for single-word section keywords.
//
// Mirrors `Lexer::resolve_keyword` in lp_parser_rs: `bounds`, `generals`,
// `bin`, `end`, ... are keywords only as the first token of a line and when
// not followed by `:` / `::` (which makes them a label). Anywhere else the
// word is an identifier, so variables and constraints may be called `bin`,
// `end`, `sos`, ... The multi-word headers (`lazy constraints`, `user cuts`,
// `general constraints`) are always keywords and stay regex tokens in
// grammar.js. Keep KEYWORDS in sync with the upstream Logos regexes.
//
// Block comments are lexed here too, so a line break before a comment still
// counts for a keyword after it on the same line (`\* note *\ Bounds`), as
// upstream skips comments when tracking line starts.

#include "tree_sitter/alloc.h"
#include "tree_sitter/parser.h"

#include <stdbool.h>
#include <string.h>

enum TokenType {
    BOUNDS_KEYWORD,
    GENERALS_KEYWORD,
    INTEGERS_KEYWORD,
    BINARIES_KEYWORD,
    SEMI_CONTINUOUS_KEYWORD,
    SOS_KEYWORD,
    END_MARKER,
    GENCONSTRS_KEYWORD,
    BLOCK_COMMENT,
};

static const struct {
    const char *word;
    enum TokenType token;
} KEYWORDS[] = {
    {"bound", BOUNDS_KEYWORD},
    {"bounds", BOUNDS_KEYWORD},
    {"gen", GENERALS_KEYWORD},
    {"general", GENERALS_KEYWORD},
    {"generals", GENERALS_KEYWORD},
    {"integer", INTEGERS_KEYWORD},
    {"integers", INTEGERS_KEYWORD},
    {"bin", BINARIES_KEYWORD},
    {"binary", BINARIES_KEYWORD},
    {"binaries", BINARIES_KEYWORD},
    {"semi", SEMI_CONTINUOUS_KEYWORD},
    {"semis", SEMI_CONTINUOUS_KEYWORD},
    {"semi-continuous", SEMI_CONTINUOUS_KEYWORD},
    {"sos", SOS_KEYWORD},
    {"end", END_MARKER},
    {"genconstr", GENCONSTRS_KEYWORD},
    {"genconstrs", GENCONSTRS_KEYWORD},
};

// Longest keyword is "semi-continuous" (15); anything longer is a name.
#define WORD_MAX 16

// State carried from a block comment to the token right after it: whether a
// line break came before or inside the comment, and the column it ended at
// (so the flag cannot leak to a later token on the same line).
typedef struct {
    bool newline_pending;
    uint32_t column;
} Scanner;

void *tree_sitter_lp_external_scanner_create(void) {
    return ts_calloc(1, sizeof(Scanner));
}

void tree_sitter_lp_external_scanner_destroy(void *payload) { ts_free(payload); }

unsigned tree_sitter_lp_external_scanner_serialize(void *payload, char *buffer) {
    memcpy(buffer, payload, sizeof(Scanner));
    return sizeof(Scanner);
}

void tree_sitter_lp_external_scanner_deserialize(void *payload, const char *buffer, unsigned length) {
    if (length == sizeof(Scanner)) {
        memcpy(payload, buffer, sizeof(Scanner));
    } else {
        *(Scanner *)payload = (Scanner){0};
    }
}

// Identifier characters (the upstream name regex, `-` included mid-name).
static bool is_name_char(int32_t c) {
    return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9') ||
           (c != 0 && c < 128 && strchr("_!#$%&(),.;?@{}~'[]|-", (int)c) != NULL);
}

static int32_t to_lower(int32_t c) { return (c >= 'A' && c <= 'Z') ? c + ('a' - 'A') : c; }

// Whether `first` and the next same-line word form the multi-word
// `general constraints` token (`general constr...` or `gen cons...`), which
// the regex token then matches instead. Like the upstream lexer's longest
// match, only a prefix of the second word is needed.
static bool follows_constraints_word(TSLexer *lexer, const char *first) {
    const char *prefix = strcmp(first, "gen") == 0 ? "cons" : strcmp(first, "general") == 0 ? "constr" : NULL;
    if (prefix == NULL) return false;
    if (lexer->lookahead != ' ' && lexer->lookahead != '\t') return false;
    while (lexer->lookahead == ' ' || lexer->lookahead == '\t') lexer->advance(lexer, false);
    for (const char *p = prefix; *p != '\0'; p++) {
        if (to_lower(lexer->lookahead) != *p) return false;
        lexer->advance(lexer, false);
    }
    return true;
}

bool tree_sitter_lp_external_scanner_scan(void *payload, TSLexer *lexer, const bool *valid_symbols) {
    Scanner *scanner = payload;
    bool at_line_start = scanner->newline_pending && lexer->get_column(lexer) == scanner->column;
    scanner->newline_pending = false;

    while (lexer->lookahead == ' ' || lexer->lookahead == '\t' || lexer->lookahead == '\r' ||
           lexer->lookahead == '\n') {
        if (lexer->lookahead == '\n') at_line_start = true;
        lexer->advance(lexer, true);
    }

    // `\*` ... `*\` (the upstream regex: ends at the first `*\`). A lone `\`
    // starts a line comment, left to the regex token.
    if (lexer->lookahead == '\\') {
        if (!valid_symbols[BLOCK_COMMENT]) return false;
        lexer->advance(lexer, false);
        if (lexer->lookahead != '*') return false;
        lexer->advance(lexer, false);
        for (;;) {
            if (lexer->eof(lexer)) return false;
            if (lexer->lookahead == '*') {
                while (lexer->lookahead == '*') lexer->advance(lexer, false);
                if (lexer->lookahead == '\\') break;
                continue;
            }
            if (lexer->lookahead == '\n') at_line_start = true;
            lexer->advance(lexer, false);
        }
        lexer->advance(lexer, false);
        scanner->newline_pending = at_line_start;
        scanner->column = lexer->get_column(lexer);
        lexer->result_symbol = BLOCK_COMMENT;
        return true;
    }
    if (!at_line_start) return false;

    char word[WORD_MAX];
    unsigned len = 0;
    while (is_name_char(lexer->lookahead)) {
        if (len == WORD_MAX - 1) return false;
        word[len++] = (char)to_lower(lexer->lookahead);
        lexer->advance(lexer, false);
    }
    word[len] = '\0';

    for (unsigned i = 0; i < sizeof(KEYWORDS) / sizeof(KEYWORDS[0]); i++) {
        if (strcmp(word, KEYWORDS[i].word) != 0) continue;
        enum TokenType token = KEYWORDS[i].token;
        if (!valid_symbols[token]) return false;
        lexer->mark_end(lexer);

        // `gen cons` / `general constraints` belong to the regex token.
        if (token == GENERALS_KEYWORD && follows_constraints_word(lexer, word)) return false;
        // `bounds:` / `end::` is a label, not a section header.
        while (lexer->lookahead == ' ' || lexer->lookahead == '\t') lexer->advance(lexer, false);
        if (lexer->lookahead == ':') return false;

        lexer->result_symbol = token;
        return true;
    }
    return false;
}
