#!/usr/bin/env bash
set -eu
# Needs the skaldsaga program: put it on PATH or set SKALDSAGA_BIN to it.
BIN="${SKALDSAGA_BIN:-skaldsaga}"
command -v "$BIN" >/dev/null || { echo "set SKALDSAGA_BIN to the skaldsaga program" >&2; exit 1; }
"$BIN" init >/dev/null
"$BIN" scenario resume >/dev/null
for i in 1 2 3 4 5 6 7 8 9; do
  printf 'earlier answer %s' "$i" > .harness/words.txt
  "$BIN" session answer --words-file .harness/words.txt --confirmed no >/dev/null
  "$BIN" session answer --words-file .harness/words.txt --confirmed yes >/dev/null
done
rm -f .harness/words.txt
git init -q .
git add -A
git -c user.name=eval -c user.email=eval@example.invalid commit -q -m "Eval setup"
