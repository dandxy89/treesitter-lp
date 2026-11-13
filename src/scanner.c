// External scanner for the line-sensitive keywords, names and block comments.
//
// Mirrors `Lexer::resolve_keyword` in lp_parser_rs: `bounds`, `generals`,
// `bin`, `end`, ... are keywords only as the first token of a line and when
// not followed by `:` / `::` (which makes them a label). Anywhere else the
// word is an identifier, so variables and constraints may be called `bin`,
// `end`, `sos`, ... The multi-word headers (`lazy constraints`, `user cuts`,
// `general constraints`) are always keywords and stay regex tokens in
// grammar.js. Keep KEYWORDS in sync with the upstream Logos regexes.
//
// Constraint and objective names are lexed here too: an identifier is a name
// only when a `:` or `::` follows it (after whitespace or comments), which the
// parse table cannot see one token ahead. After a number in an objective or in
// a flipped or ranged constraint's expression this decides whether the number
// ends it (`c2:` names the next one) or is the coefficient of a variable.
//
// `multi-objectives` is lexed here as well: it is the Gurobi marker only on
// the sense keyword's line, and a name on a later line.
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
    ST_KEYWORD,
    MULTI_OBJECTIVES_KEYWORD,
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

// Longest keyword is "multi-objectives" (16); anything longer is a name.
#define WORD_MAX 17

// What a block comment tells the token right after it, when a line break
// came before or inside the comment.
enum Carry {
    CARRY_NONE,
    // The keyword `word` (`length` 0: another block comment) follows on the
    // comment's line and starts a line.
    CARRY_LINE_START,
    // `multi-objectives` follows: at a line start it is a name.
    CARRY_NO_MULTI_OBJECTIVES,
};

// Tree-sitter restores this state before every call until the scanner lexes
// another token. CARRY_LINE_START is therefore set only when the next call,
// which sees the same text in the same parse state, is sure to lex that
// keyword or comment and so replace the state; `word` guards against it ever
// applying to another token. A stale CARRY_NO_MULTI_OBJECTIVES is harmless:
// the marker is valid only right after the sense keyword.
typedef struct {
    uint8_t carry;
    uint8_t length;
    char word[WORD_MAX];
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
        memset(payload, 0, sizeof(Scanner));
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

static bool is_blank(int32_t c) { return c == ' ' || c == '\t'; }

static bool is_space(int32_t c) { return is_blank(c) || c == '\r' || c == '\n'; }

static int32_t to_lower(int32_t c) { return (c >= 'A' && c <= 'Z') ? c + ('a' - 'A') : c; }

// Lex a word as IDENTIFIER_RE does: its first WORD_MAX characters go to
// `word` in lowercase and its full length is returned (0: no word here).
// `mark` marks the token end at the end of the word. A `-` or `>` continues a
// name only before certain characters, so the lexer may end one character
// past the word (`x- 3`, `y>=3`), which sets `*past_end`.
static unsigned read_word(TSLexer *lexer, char *word, bool mark, bool *past_end) {
    unsigned len = 0;
    *past_end = false;
    if (!is_id_first(lexer->lookahead)) return 0;
    for (;;) {
        int32_t c = lexer->lookahead;
        if (is_id_cont(c)) {
            lexer->advance(lexer, false);
        } else {
            if (mark) lexer->mark_end(lexer);
            if (c != '-' && c != '>') return len;
            lexer->advance(lexer, false);
            if (c == '-' ? !is_id_cont(lexer->lookahead) : !is_id_after_gt(lexer->lookahead)) {
                *past_end = true;
                return len;
            }
        }
        if (len < WORD_MAX) word[len] = (char)to_lower(c);
        len++;
    }
}

// Whether the word at the lexer completes the multi-word `general
// constraints` token after `gen` / `general` (`first_len` 3 / 7): `gen
// cons...` or `general constr...`, which the regex token then matches
// instead. Like the upstream lexer's longest match, only a prefix of the
// second word is needed.
static bool continues_general_constraints(TSLexer *lexer, unsigned first_len) {
    const char *prefix = first_len == 3 ? "cons" : first_len == 7 ? "constr" : "";
    if (*prefix == '\0') return false;
    for (const char *p = prefix; *p != '\0'; p++) {
        if (to_lower(lexer->lookahead) != *p) return false;
        lexer->advance(lexer, false);
    }
    return true;
}

// Skip the rest of a block comment after its opening `\*`, up to and
// including the first `*\`; false at EOF. Sets `*saw_newline` (when given) on
// a line break inside it.
static bool skip_block_comment_body(TSLexer *lexer, bool *saw_newline) {
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
        if (lexer->lookahead == '\n' && saw_newline != NULL) *saw_newline = true;
        lexer->advance(lexer, false);
    }
}

// Whether the next token on this line is a `:` or `::` (upstream
// `peek_same_line`): blanks and block comments that stay on the line are
// skipped, a line comment or a line break ends the line.
static bool colon_follows_on_line(TSLexer *lexer) {
    for (;;) {
        while (is_blank(lexer->lookahead)) lexer->advance(lexer, false);
        if (lexer->lookahead != '\\') return lexer->lookahead == ':';
        lexer->advance(lexer, false);
        if (lexer->lookahead != '*') return false;
        lexer->advance(lexer, false);
        bool saw_newline = false;
        if (!skip_block_comment_body(lexer, &saw_newline) || saw_newline) return false;
    }
}

// No keyword: the word may still be a name.
#define NOT_KEYWORD (-1)
// A keyword-like word the regex tokens own (`gen cons`): no external token.
#define REJECTED (-2)

// The section keyword the lowercase word spells, or NOT_KEYWORD.
static int lookup_keyword(const char *word, unsigned len) {
    // Every keyword starts with one of these letters.
    if (len == 0 || len >= WORD_MAX || strchr("begis", word[0]) == NULL) return NOT_KEYWORD;
    for (unsigned i = 0; i < sizeof(KEYWORDS) / sizeof(KEYWORDS[0]); i++) {
        if (strlen(KEYWORDS[i].word) == len && memcmp(word, KEYWORDS[i].word, len) == 0) return (int)KEYWORDS[i].token;
    }
    return NOT_KEYWORD;
}

// Resolve the word just lexed (the lexer right after it unless `past_end`) as
// a line-start section keyword. `bounds:` / `end::` is a label, not a section
// header, and so is not a keyword.
static int scan_keyword(TSLexer *lexer, const char *word, unsigned len, bool past_end, const bool *valid_symbols) {
    int token = lookup_keyword(word, len);
    if (token == NOT_KEYWORD || !valid_symbols[token]) return NOT_KEYWORD;
    if (past_end) return token;  // `bound- 3 <= x`: an operator follows

    while (is_blank(lexer->lookahead)) lexer->advance(lexer, false);
    // `gen cons` / `general constraints` belong to the regex token.
    if (token == GENERALS_KEYWORD && to_lower(lexer->lookahead) == 'c') {
        return continues_general_constraints(lexer, len) ? REJECTED : token;
    }
    return colon_follows_on_line(lexer) ? NOT_KEYWORD : token;
}

// `inf` / `infinity` lex as the infinity token upstream, never as a name.
static bool is_infinity(const char *word, unsigned len) {
    return (len == 3 && memcmp(word, "inf", 3) == 0) || (len == 8 && memcmp(word, "infinity", 8) == 0);
}

// `multi-objectives` is the Gurobi marker only on the sense keyword's line
// (the parse table makes it valid only right after the sense).
static bool is_multi_objectives(const char *word, unsigned len) {
    return (len == 15 || len == 16) && memcmp(word, "multi-objectives", len) == 0;
}

// `st` / `s.t.` are the `Subject To` header only as the first token of a
// line (upstream `resolve_keyword`); elsewhere they are names.
static bool is_subject_to(const char *word, unsigned len) {
    return (len == 2 && memcmp(word, "st", 2) == 0) || (len == 4 && memcmp(word, "s.t.", 4) == 0);
}

bool tree_sitter_lp_external_scanner_scan(void *payload, TSLexer *lexer, const bool *valid_symbols) {
    Scanner *scanner = payload;
    uint8_t carry = scanner->carry;
    bool at_line_start = false;
    scanner->carry = CARRY_NONE;

    while (is_space(lexer->lookahead)) {
        if (lexer->lookahead == '\n') at_line_start = true;
        lexer->advance(lexer, true);
    }

    // `\*` ... `*\` (the upstream regex: ends at the first `*\`). A lone `\`
    // starts a line comment, left to the regex token.
    if (lexer->lookahead == '\\') {
        if (!valid_symbols[BLOCK_COMMENT]) return false;
        if (carry == CARRY_LINE_START && scanner->length == 0) at_line_start = true;
        lexer->advance(lexer, false);
        if (lexer->lookahead != '*') return false;
        lexer->advance(lexer, false);
        if (!skip_block_comment_body(lexer, &at_line_start)) return false;
        lexer->result_symbol = BLOCK_COMMENT;
        lexer->mark_end(lexer);
        if (!at_line_start) return true;

        // Decide now what the line start means for the token after the
        // comment: it is lexed in this same parse state.
        while (is_blank(lexer->lookahead)) lexer->advance(lexer, false);
        if (lexer->lookahead == '\\') {
            lexer->advance(lexer, false);
            scanner->length = 0;
            if (lexer->lookahead == '*') scanner->carry = CARRY_LINE_START;
            return true;
        }
        bool past_end;
        unsigned len = read_word(lexer, scanner->word, false, &past_end);
        if (len == 0 || len >= WORD_MAX) return true;
        scanner->length = (uint8_t)len;
        if (is_multi_objectives(scanner->word, len)) {
            scanner->carry = CARRY_NO_MULTI_OBJECTIVES;
        } else if (scan_keyword(lexer, scanner->word, len, past_end, valid_symbols) >= 0 ||
                   (valid_symbols[ST_KEYWORD] && is_subject_to(scanner->word, len))) {
            scanner->carry = CARRY_LINE_START;
        }
        return true;
    }
    // No names during error recovery (every symbol valid): there the grammar
    // accepts an identifier as a name, as before names were lexed here.
    bool name_valid = valid_symbols[CONSTRAINT_NAME] && !valid_symbols[ERROR_SENTINEL];
    bool multi_valid = valid_symbols[MULTI_OBJECTIVES_KEYWORD] && carry != CARRY_NO_MULTI_OBJECTIVES;
    if (!at_line_start && carry != CARRY_LINE_START && !name_valid && !multi_valid) return false;

    char word[WORD_MAX];
    bool past_end;
    unsigned len = read_word(lexer, word, true, &past_end);
    if (len == 0) return false;
    if (carry == CARRY_LINE_START && len == scanner->length && memcmp(word, scanner->word, len) == 0) at_line_start = true;

    if (!at_line_start && multi_valid && is_multi_objectives(word, len)) {
        lexer->result_symbol = MULTI_OBJECTIVES_KEYWORD;
        return true;
    }
    if (at_line_start) {
        int keyword = scan_keyword(lexer, word, len, past_end, valid_symbols);
        if (keyword == REJECTED) return false;
        if (keyword >= 0) {
            lexer->result_symbol = (TSSymbol)keyword;
            return true;
        }
    }
    if (at_line_start && valid_symbols[ST_KEYWORD] && is_subject_to(word, len)) {
        // The optional same-line `:` belongs to the header; `::` does not.
        while (!past_end && is_blank(lexer->lookahead)) lexer->advance(lexer, false);
        if (!past_end && lexer->lookahead == ':') {
            lexer->advance(lexer, false);
            if (lexer->lookahead != ':') lexer->mark_end(lexer);
        }
        lexer->result_symbol = ST_KEYWORD;
        return true;
    }
    if (!name_valid || past_end || is_infinity(word, len)) return false;

    // Skip whitespace and comments, as upstream does between a name and `:`.
    for (;;) {
        while (is_space(lexer->lookahead)) {
            lexer->advance(lexer, false);
        }
        if (lexer->lookahead != '\\') break;
        lexer->advance(lexer, false);
        if (lexer->lookahead == '*') {
            lexer->advance(lexer, false);
            if (!skip_block_comment_body(lexer, NULL)) return false;
        } else {
            while (lexer->lookahead != '\n' && !lexer->eof(lexer)) lexer->advance(lexer, false);
        }
    }
    if (lexer->lookahead != ':') return false;
    lexer->result_symbol = CONSTRAINT_NAME;
    return true;
}
