#!/bin/sh
# Fails if the generated parser in src/ is out of step with grammar.js.
# Generates into a scratch copy and compares; the working tree is untouched.
# `src/scanner.c` is hand-written and not checked. Uses `tree-sitter` from
# node_modules or PATH.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"

if [ -x node_modules/.bin/tree-sitter ]; then
  ts=$root/node_modules/.bin/tree-sitter
elif command -v tree-sitter >/dev/null 2>&1; then
  ts=$(command -v tree-sitter)
else
  echo "check-generated: tree-sitter CLI not found (run npm ci)" >&2
  exit 2
fi

scratch=$(mktemp -d)
trap 'rm -rf "$scratch"' EXIT INT TERM

cp grammar.js tree-sitter.json package.json "$scratch"/
mkdir "$scratch/src"
# The external scanner is read by `generate` only through grammar.js.
cp src/scanner.c "$scratch/src/"
(cd "$scratch" && "$ts" generate >/dev/null 2>&1) || {
  echo "check-generated: tree-sitter generate failed" >&2
  exit 2
}

status=0
for file in parser.c grammar.json node-types.json tree_sitter/parser.h tree_sitter/alloc.h tree_sitter/array.h; do
  if [ ! -f "src/$file" ]; then
    echo "check-generated: src/$file is missing" >&2
    status=1
  elif ! cmp -s "src/$file" "$scratch/src/$file"; then
    echo "check-generated: src/$file is stale" >&2
    diff -u "src/$file" "$scratch/src/$file" | head -40 >&2 || true
    status=1
  fi
done

if [ "$status" -ne 0 ]; then
  echo "check-generated: run 'npx tree-sitter generate' and commit src/" >&2
  exit 1
fi
echo "check-generated: src/ is in sync with grammar.js"
