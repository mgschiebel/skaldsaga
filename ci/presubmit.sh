#!/bin/sh
# Every pre-merge check in one place (docs/handoff/security.md, "Application
# security tooling"; appsec-tooling-research.md, Decided item 6). Build sessions
# run it before every push, and CI runs the same checks through the same
# script, so the local run and the gate cannot drift apart.
#
#   sh ci/presubmit.sh                 run every check
#   sh ci/presubmit.sh fmt clippy      run only the named checks
#
# Checks: fmt, clippy, test, shellcheck, deny, license-sync, license-elections,
# release-files, exceptions (every suppression is on docs/security/exceptions.toml
# and unexpired; ci/check-exceptions.py), fuzz (a short pass: needs cargo-fuzz and a nightly toolchain;
# FUZZ_SECONDS and FUZZ_TOOLCHAIN are read by ci/fuzz-smoke.sh).
#
# A check that cannot run in this environment is reported as NOT RUN and the
# script exits non-zero, so nothing is skipped silently. In CI the same
# situation is an error. When a cloud session cannot run a check (for example,
# the ShellCheck tool is not installed), say so in the pull request
# description: CI is the only place that check runs.
set -u

cd "$(dirname "$0")/.." || exit 1

all="fmt clippy test shellcheck deny license-sync license-elections release-files exceptions fuzz"
if [ "$#" -gt 0 ]; then checks="$*"; else checks="$all"; fi

failed=""
notrun=""

# Run one named check. `need` is a command the check requires.
run() {
  name=$1
  need=$2
  shift 2
  if [ -n "$need" ] && ! command -v "$need" > /dev/null 2>&1; then
    echo "== $name: NOT RUN ($need is not installed here)"
    notrun="$notrun $name"
    return
  fi
  echo "== $name"
  if "$@"; then
    echo "== $name: ok"
  else
    echo "== $name: FAILED"
    failed="$failed $name"
  fi
}

for check in $checks; do
  case "$check" in
    # The fuzz crate sits outside the workspace (it needs nightly to run), so
    # the format and lint checks name it on their own; both work on stable.
    fmt) run fmt cargo sh -c 'cargo fmt --all --check && cargo fmt --manifest-path fuzz/Cargo.toml --check' ;;
    clippy) run clippy cargo sh -c 'cargo clippy --all-targets --locked -- -D warnings && cargo clippy --manifest-path fuzz/Cargo.toml --all-targets --locked -- -D warnings' ;;
    test) run test cargo cargo test --locked ;;
    # Every shell script the plugin ships or CI runs.
    shellcheck) run shellcheck shellcheck shellcheck -f gcc plugin/scripts/skaldsaga ci/*.sh ;;
    deny) run deny cargo-deny cargo deny --locked check ;;
    license-sync) run license-sync "" sh ci/check-license-sync.sh ;;
    license-elections) run license-elections python3 python3 -I ci/check-license-elections.py ;;
    release-files) run release-files python3 python3 -I ci/release-files.py check ;;
    exceptions) run exceptions python3 sh -c 'python3 -I ci/check-exceptions.py && python3 -I ci/test-check-exceptions.py' ;;
    fuzz) run fuzz cargo-fuzz sh ci/fuzz-smoke.sh ;;
    *)
      echo "presubmit: unknown check '$check' (known: $all)" >&2
      exit 2
      ;;
  esac
done

echo
if [ -n "$failed" ]; then echo "FAILED:$failed"; fi
if [ -n "$notrun" ]; then echo "NOT RUN:$notrun (CI runs these; say so in the pull request description)"; fi
if [ -z "$failed" ] && [ -z "$notrun" ]; then
  echo "All checks passed."
  exit 0
fi
exit 1
