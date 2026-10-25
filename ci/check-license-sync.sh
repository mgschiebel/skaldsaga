#!/bin/sh
# Fails when the license allowlist in docs/security/license-policy.md and the
# one in deny.toml ([licenses] allow) are not the same set. The policy file is
# the list people read; deny.toml is the list the tool enforces. Run from
# anywhere; it finds the repository root itself.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
policy="$root/docs/security/license-policy.md"
deny="$root/deny.toml"

# Bullets that are a bare SPDX id, between "## The allowlist" and the next heading.
from_policy=$(awk '
  /^## /       { in_list = ($0 == "## The allowlist"); next }
  in_list && /^- [A-Za-z0-9.+-]+$/ { sub(/^- /, ""); print }
' "$policy" | sort)

# Quoted ids inside the multi-line `allow = [ ... ]` of the [licenses] table only
# (the per-crate exceptions are single-line and live in other tables).
from_deny=$(awk '
  /^\[/                                  { sec = $0 }
  sec == "[licenses]" && /^allow = \[$/  { in_list = 1; next }
  in_list && /^\]/                       { in_list = 0 }
  in_list { gsub(/[ ",]/, ""); if ($0 != "") print }
' "$deny" | sort)

if [ -z "$from_policy" ] || [ -z "$from_deny" ]; then
  echo "check-license-sync: could not read one of the lists (policy: $(printf '%s' "$from_policy" | wc -l) lines, deny.toml: $(printf '%s' "$from_deny" | wc -l) lines)" >&2
  exit 1
fi

if [ "$from_policy" = "$from_deny" ]; then
  echo "License allowlist matches between license-policy.md and deny.toml ($(printf '%s\n' "$from_deny" | wc -l | tr -d ' ') licenses)."
  exit 0
fi

echo "License allowlists differ between docs/security/license-policy.md and deny.toml:" >&2
printf '%s\n' "$from_policy" > "${TMPDIR:-/tmp}/license-sync.policy.$$"
printf '%s\n' "$from_deny" > "${TMPDIR:-/tmp}/license-sync.deny.$$"
diff "${TMPDIR:-/tmp}/license-sync.policy.$$" "${TMPDIR:-/tmp}/license-sync.deny.$$" >&2 || true
rm -f "${TMPDIR:-/tmp}/license-sync.policy.$$" "${TMPDIR:-/tmp}/license-sync.deny.$$"
echo "(< is only in license-policy.md, > is only in deny.toml)" >&2
exit 1
