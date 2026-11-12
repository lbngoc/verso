#!/usr/bin/env bash
# Prints the release notes for a tag: the matching CHANGELOG section plus a signed line per OS
# (007 T011, FR-011). The release workflow writes this to --notes-file.
#
# Usage: scripts/release-notes.sh <tag>
#
# <tag> is vMAJOR.MINOR.PATCH or vMAJOR.MINOR.PATCH-suffix; a pre-release shares the section of its
# core version (v0.1.0-rc.1 reads "## 0.1.0").
# Signing state comes from MACOS_SIGNED and WINDOWS_SIGNED (yes/true/1 mean signed; anything else is no).
set -euo pipefail

usage() {
  sed -n '2,11p' "$0"
  exit 2
}

[[ $# -eq 1 ]] || usage
tag="$1"
full="${tag#v}"
version="${full%%-*}"
file="${CHANGELOG_FILE:-docs/public/CHANGELOG.md}"

[[ -n "$version" ]] || usage
[[ -f "$file" ]] || {
  echo "release-notes: $file does not exist" >&2
  exit 1
}

section=$(awk -v want="$version" '
  /^##[[:space:]]/ {
    if (printing) { exit }
    line = $0
    sub(/^##[[:space:]]+/, "", line)
    sub(/[[:space:]].*$/, "", line)
    if (line == want) { printing = 1; next }
  }
  printing { print }
' "$file")

# Drop the blank lines the section starts and ends with.
section=$(printf '%s\n' "$section" | sed -e '/./,$!d')
section=$(printf '%s\n' "$section" | sed -e :a -e '/^\n*$/{$d;N;ba' -e '}')

if [[ -z "${section//[[:space:]]/}" ]]; then
  echo "release-notes: $file has no '## $version' section" >&2
  exit 1
fi

signed() {
  case "${1:-}" in
    yes | YES | true | TRUE | 1) echo yes ;;
    *) echo no ;;
  esac
}

printf '%s\n' "$section"
echo
echo "Signed (macOS): $(signed "${MACOS_SIGNED:-no}")"
echo "Signed (Windows): $(signed "${WINDOWS_SIGNED:-no}")"
