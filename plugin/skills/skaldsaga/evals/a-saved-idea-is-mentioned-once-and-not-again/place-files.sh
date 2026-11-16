#!/usr/bin/env bash
set -eu
# Needs the skaldsaga program: put it on PATH or set SKALDSAGA_BIN to it.
BIN="${SKALDSAGA_BIN:-skaldsaga}"
command -v "$BIN" >/dev/null || { echo "set SKALDSAGA_BIN to the skaldsaga program" >&2; exit 1; }
"$BIN" init >/dev/null
"$BIN" scenario resume >/dev/null
printf 'A flooded underpass cuts their route in half.\n' > .harness/idea-text.txt
"$BIN" idea add --text-file .harness/idea-text.txt --origin brainstorm --touches complications >/dev/null
rm -f .harness/idea-text.txt

rm -f .harness/words.txt .harness/meter.json
git init -q .
git add -A
git -c user.name=eval -c user.email=eval@example.invalid commit -q -m "Eval setup"
