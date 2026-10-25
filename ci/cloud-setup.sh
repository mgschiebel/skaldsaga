#!/bin/sh
# Environment setup script for the author's Claude Code Project (phone testing).
# Paste into Project settings > Environment > Setup script. It runs before each
# cloud session starts.
#
# It no longer makes a Skaldsaga project. Earlier versions created
# /home/claude/.harness so the session-start hook had something to find, but
# that left a stray, empty project behind: claude.ai/code on the web starts the
# session inside the attached repository (for example /home/user/<repo>), and
# the mobile app starts it in /home/claude with the repository one level down.
# The hook handles both layouts on its own, and `/skaldsaga` sets up and wires
# the meter in the repository itself (0.3.2 re-test notes, docs/tests/).
set -eu

# Let cargo reach crates.io (see CLAUDE.md), so build sessions skip that step.
mkdir -p /root/.local/bin
cat > /root/.local/bin/cargo <<'EOF'
#!/bin/sh
NO_PROXY= no_proxy= exec "$HOME/.cargo/bin/cargo" "$@"
EOF
chmod +x /root/.local/bin/cargo

# Keep Claude session links out of commits and pull requests (SR-03). The repo's
# .claude/settings.json only applies to sessions that start inside the repository
# (the mobile app starts in /home/claude), so set it at user level too, where it
# applies wherever the session starts. An existing settings file is merged, never
# replaced; one that isn't valid JSON is left alone.
mkdir -p /root/.claude
python3 -I - <<'PY'
import json, os
path = "/root/.claude/settings.json"
settings = {}
if os.path.exists(path):
    try:
        with open(path) as f:
            settings = json.load(f)
    except ValueError:
        raise SystemExit(0)
    if not isinstance(settings, dict):
        raise SystemExit(0)
attribution = settings.get("attribution")
if not isinstance(attribution, dict):
    attribution = {}
attribution["sessionUrl"] = False
settings["attribution"] = attribution
with open(path, "w") as f:
    json.dump(settings, f, indent=2)
    f.write("\n")
PY
