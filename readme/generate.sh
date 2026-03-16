#!/usr/bin/env bash
#
# Generates README files from templates + shared partials.
# Usage: ./readme/generate.sh [--check]
#
# With --check, exits non-zero if any generated README differs from what's on disk.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PARTIALS_DIR="$REPO_ROOT/readme/partials"
TEMPLATES_DIR="$REPO_ROOT/readme/templates"
CHECK_MODE=false

if [[ "${1:-}" == "--check" ]]; then
  CHECK_MODE=true
fi

# Render a template by replacing {{> partial-name}} with partial file contents.
render_template() {
  local template="$1"
  local output=""
  while IFS= read -r line || [[ -n "$line" ]]; do
    if [[ "$line" =~ ^\{\{\>\ ([a-zA-Z0-9_-]+)\}\}$ ]]; then
      local partial_name="${BASH_REMATCH[1]}"
      local partial_file="$PARTIALS_DIR/${partial_name}.md"
      if [[ ! -f "$partial_file" ]]; then
        echo "Error: partial '$partial_name' not found at $partial_file" >&2
        exit 1
      fi
      output+="$(cat "$partial_file")"$'\n'
    else
      output+="$line"$'\n'
    fi
  done < "$template"
  printf '%s' "${output%$'\n'}"
}

# template_name:output_path pairs
TARGETS="node:bindings/node/README.md
python:bindings/python/README.md
ows:ows/README.md
ows:ows/crates/ows-core/README.md
ows:ows/crates/ows-signer/README.md
ows:ows/crates/ows-lib/README.md
ows:ows/crates/ows-cli/README.md"

failures=0

while IFS=: read -r name relpath; do
  template="$TEMPLATES_DIR/${name}.md"
  target="$REPO_ROOT/$relpath"

  if [[ ! -f "$template" ]]; then
    echo "Warning: template $template not found, skipping" >&2
    continue
  fi

  rendered="$(render_template "$template")"

  if $CHECK_MODE; then
    if [[ ! -f "$target" ]]; then
      echo "FAIL: $relpath does not exist (expected from $template)"
      failures=$((failures + 1))
    elif ! diff -q <(printf '%s\n' "$rendered") "$target" > /dev/null 2>&1; then
      echo "FAIL: $relpath is out of date — run ./readme/generate.sh"
      diff --unified <(printf '%s\n' "$rendered") "$target" || true
      failures=$((failures + 1))
    else
      echo "OK: $relpath"
    fi
  else
    printf '%s\n' "$rendered" > "$target"
    echo "Generated: $relpath"
  fi
done <<< "$TARGETS"

if $CHECK_MODE && [[ $failures -gt 0 ]]; then
  echo ""
  echo "$failures README(s) out of date. Run: ./readme/generate.sh"
  exit 1
fi