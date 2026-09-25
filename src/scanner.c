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
// Constraint names are lexed here too: an identifier is a name only when a
// `:` or `::` follows it (after whitespace or comments), which the parse
// table cannot see one token ahead. After a number in a flipped or ranged
// constraint's expression this decides whether the number ends it (`c2:`
// names the next constraint) or is the coefficient of a variable.
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
    CONSTRAINT_NAME,
    ERROR_SENTINEL,
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

static bool is_letter(int32_t c) { return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z'); }

// The three character classes of IDENTIFIER_RE in grammar.js.
static bool is_id_first(int32_t c) {
    return is_letter(c) || (c != 0 && c < 128 && strchr("_!#$%&(),.;?@{}~'[]", (int)c) != NULL);
}

static bool is_id_cont(int32_t c) {
    return is_letter(c) || (c >= '0' && c <= '9') || c == '|' || is_id_first(c);
}

static bool is_id_after_gt(int32_t c) {
    return is_letter(c) || (c != 0 && c < 128 && strchr("_!#$%&(),;?@{}~'|[]", (int)c) != NULL);
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

// No keyword: the word may still be a name.
#define NOT_KEYWORD (-1)
// A keyword-like word the regex tokens own (`gen cons`): no external token.
#define REJECTED (-2)

// Look up the lowercase word just lexed (`len` characters, the lexer right
// after it) as a line-start section keyword. `bounds:` / `end::` is a label,
// not a section header, and so is not a keyword.
static int scan_keyword(TSLexer *lexer, const char *word, unsigned len, const bool *valid_symbols) {
    // Every keyword starts with one of these letters.
    if (len >= WORD_MAX || strchr("begis", word[0]) == NULL) return NOT_KEYWORD;
    char text[WORD_MAX];
    memcpy(text, word, len);
    text[len] = '\0';
    for (unsigned i = 0; i < sizeof(KEYWORDS) / sizeof(KEYWORDS[0]); i++) {
        if (strcmp(text, KEYWORDS[i].word) != 0) continue;
        enum TokenType token = KEYWORDS[i].token;
        if (!valid_symbols[token]) return NOT_KEYWORD;
        lexer->mark_end(lexer);

        // `gen cons` / `general constraints` belong to the regex token.
        if (token == GENERALS_KEYWORD && follows_constraints_word(lexer, text)) return REJECTED;
        while (lexer->lookahead == ' ' || lexer->lookahead == '\t') lexer->advance(lexer, false);
        if (lexer->lookahead == ':') return NOT_KEYWORD;
        return (int)token;
    }
    return NOT_KEYWORD;
}

// `inf` / `infinity` lex as the infinity token upstream, never as a name.
static bool is_infinity(const char *word, unsigned len) {
    return (len == 3 && memcmp(word, "inf", 3) == 0) || (len == 8 && memcmp(word, "infinity", 8) == 0);
}

// Skip the rest of a block comment after its opening `\*`; false at EOF.
static bool skip_block_comment_body(TSLexer *lexer) {
    lexer->advance(lexer, false);
    for (;;) {
        if (lexer->eof(lexer)) return false;
        if (lexer->lookahead == '*') {
            while (lexer->lookahead == '*') lexer->advance(lexer, false);
            if (lexer->lookahead == '\\') {
                lexer->advance(lexer, false);
                return true;
            }
            continue;
        }
        lexer->advance(lexer, false);
    }
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
    // No names during error recovery (every symbol valid): there the grammar
    // accepts an identifier as a name, as before names were lexed here.
    bool name_valid = valid_symbols[CONSTRAINT_NAME] && !valid_symbols[ERROR_SENTINEL];
    if (!at_line_start && !name_valid) return false;

    // Lex a word as IDENTIFIER_RE does, keeping its lowercase spelling for
    // the keyword lookup. A keyword can only end at `>` or at the first
    // character that is not part of a name.
    if (!is_id_first(lexer->lookahead)) return false;
    char word[WORD_MAX];
    unsigned len = 0;
    for (;;) {
        int32_t c = lexer->lookahead;
        if (is_id_cont(c)) {
            if (len < WORD_MAX) word[len] = (char)to_lower(c);
            len++;
            lexer->advance(lexer, false);
            continue;
        }
        if (c == '-') {
            lexer->advance(lexer, false);
            if (!is_id_cont(lexer->lookahead)) return false;  // `x-` : not a name, nor a keyword
            if (len < WORD_MAX) word[len] = '-';
            len++;
            continue;
        }
        if (c == '>') {
            if (at_line_start) {
                int keyword = scan_keyword(lexer, word, len, valid_symbols);
                if (keyword >= 0) {
                    lexer->result_symbol = (TSSymbol)keyword;
                    return true;
                }
            }
            lexer->advance(lexer, false);
            if (!is_id_after_gt(lexer->lookahead)) return false;  // `y>=` : the name ends before `>`
            if (len < WORD_MAX) word[len] = '>';
            len++;
            continue;
        }
        break;
    }
    lexer->mark_end(lexer);

    if (at_line_start) {
        int keyword = scan_keyword(lexer, word, len, valid_symbols);
        if (keyword == REJECTED) return false;
        if (keyword >= 0) {
            lexer->result_symbol = (TSSymbol)keyword;
            return true;
        }
    }
    if (!name_valid || is_infinity(word, len)) return false;

    // Skip whitespace and comments, as upstream does between a name and `:`.
    for (;;) {
        while (lexer->lookahead == ' ' || lexer->lookahead == '\t' || lexer->lookahead == '\r' ||
               lexer->lookahead == '\n') {
            lexer->advance(lexer, false);
        }
        if (lexer->lookahead != '\\') break;
        lexer->advance(lexer, false);
        if (lexer->lookahead == '*') {
            if (!skip_block_comment_body(lexer)) return false;
        } else {
            while (lexer->lookahead != '\n' && !lexer->eof(lexer)) lexer->advance(lexer, false);
        }
    }
    if (lexer->lookahead != ':') return false;
    lexer->result_symbol = CONSTRAINT_NAME;
    return true;
}
