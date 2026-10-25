#!/bin/sh
# A short fuzz pass over every fuzz target (appsec-tooling-research.md, step 5).
# Called by `sh ci/presubmit.sh fuzz`, locally and in CI.
#
#   FUZZ_SECONDS    seconds per target (default 10)
#   FUZZ_TOOLCHAIN  the nightly toolchain to use (default nightly; CI pins a date)
#
# Each target first replays the committed seeds in fuzz/seeds/<target>/, so an
# input that once crashed a loader stays a regression test: when fuzzing finds
# a crash, fix the loader and add the input there.
set -eu

cd "$(dirname "$0")/../fuzz"

seconds=${FUZZ_SECONDS:-10}
toolchain=${FUZZ_TOOLCHAIN:-nightly}

targets=$(cargo "+$toolchain" fuzz list)
if [ -z "$targets" ]; then
  echo "fuzz-smoke: no fuzz targets found" >&2
  exit 1
fi

for target in $targets; do
  echo "-- fuzz target: $target ($seconds s)"
  mkdir -p "corpus/$target"
  if [ -d "seeds/$target" ]; then cp -R "seeds/$target/." "corpus/$target/"; fi
  # A crash exits non-zero and leaves the input under fuzz/artifacts/.
  cargo "+$toolchain" fuzz run "$target" -- "-max_total_time=$seconds" -timeout=10 > "$target.log" 2>&1 || {
    tail -n 40 "$target.log"
    rm -f "$target.log"
    echo "fuzz-smoke: $target found a crash (see fuzz/artifacts/$target/)" >&2
    exit 1
  }
  grep -E '^Done ' "$target.log" || true
  rm -f "$target.log"
done
