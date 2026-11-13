#!/usr/bin/env bash
# Colour sweep (spec 009 T059): components and stylesheets read colours from tokens.css.
# Literal colours are allowed only in the token files; --danger only on errors and destructive actions.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

fail=0
allow_prefixes=("src/styles/tokens.css" "src/lib/theme-tokens.ts")

files=$(find src -type f \( -name '*.svelte' -o -name '*.css' -o -name '*.ts' \) \
  ! -name '*.test.ts' ! -name 'theme-tokens.ts' ! -name 'tokens.css')

while IFS= read -r file; do
  [ -n "$file" ] || continue
  matches=$(grep -nE '#[0-9a-fA-F]{3,8}\b|rgba?\(' "$file" || true)
  if [ -n "$matches" ]; then
    echo "check-colors: literal colour in $file" >&2
    echo "$matches" >&2
    fail=1
  fi
done <<< "$files"

# --danger is reserved for errors and destructive actions.
while IFS= read -r line; do
  file="${line%%:*}"
  case "$file" in
    src/lib/components/ErrorBanner.svelte|\
    src/lib/components/CaptureFailed.svelte|\
    src/lib/components/ShortcutRecorder.svelte|\
    src/routes/settings/sections/Shortcuts.svelte|\
    src/routes/settings/+page.svelte|\
    src/routes/onboarding/+page.svelte) ;;
    *)
      echo "check-colors: --danger outside an error surface: $line" >&2
      fail=1
      ;;
  esac
done <<< "$(grep -rn --include='*.svelte' --include='*.css' --include='*.ts' -- '--danger' src | grep -v 'tokens.css' || true)"

if [ "$fail" -ne 0 ]; then
  exit 1
fi
echo "check-colors: every colour comes from tokens.css"
