#!/bin/sh
# Assembles the installable plugin: the plugin folder plus the compiled
# binary, from one source version. Used by CI on every push (so pull requests
# test exactly what a release ships) and by the release workflow.
#
# Usage: sh ci/assemble-dist.sh OUT_DIR BINARY [PLATFORM]
set -eu

out=$1
binary=$2
platform=${3:-linux-x64}
root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)

crate_version=$(cargo metadata --no-deps --format-version 1 --manifest-path "$root/Cargo.toml" \
  | python3 -c 'import json,sys; print([p["version"] for p in json.load(sys.stdin)["packages"] if p["name"]=="skaldsaga"][0])')
plugin_version=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["version"])' \
  "$root/plugin/.claude-plugin/plugin.json")
if [ "$crate_version" != "$plugin_version" ]; then
  echo "Version mismatch: Cargo.toml says $crate_version, plugin.json says $plugin_version" >&2
  exit 1
fi

rm -rf "$out"
mkdir -p "$out"
cp -a "$root/plugin/." "$out/"
# The skill's eval cases are for building it, not for writers.
rm -rf "$out/skills/skaldsaga/evals"
mkdir -p "$out/scripts/bin/$platform"
cp "$binary" "$out/scripts/bin/$platform/skaldsaga"
chmod +x "$out/scripts/skaldsaga" "$out/scripts/bin/$platform/skaldsaga"
echo "$crate_version" > "$out/scripts/VERSION"
cp "$root/LICENSE" "$root/NOTICE" "$out/"
# The licenses and notices of the crates compiled into the binary, which the
# permissive licenses require us to pass on with it.
python3 -I "$root/ci/release-files.py" notices --output "$out/THIRD-PARTY-NOTICES.md"

echo "Assembled Skaldsaga $crate_version ($platform) in $out"
