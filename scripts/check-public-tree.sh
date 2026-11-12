#!/usr/bin/env bash
# Fails when a public-repository checkout holds anything beyond the stage 1 documents (007 T012,
# SC-008): no source files, no specs or agent folders, no secrets.
#
# Usage: scripts/check-public-tree.sh <checkout>
#        scripts/check-public-tree.sh --self-test
#
# Allowed at the top level and nothing else (contracts/public-project.md):
#   README.md README_vi.md INSTALL.md TROUBLESHOOTING.md PRIVACY.md TERMS.md CHANGELOG.md THIRD_PARTY.md SECURITY.md
# Allowed in .github/ISSUE_TEMPLATE (revision 3, FR-019):
#   bug_report.yml feature_request.yml config.yml
set -euo pipefail

ALLOWED=(
  README.md
  README_vi.md
  INSTALL.md
  TROUBLESHOOTING.md
  PRIVACY.md
  TERMS.md
  CHANGELOG.md
  THIRD_PARTY.md
  SECURITY.md
)

ALLOWED_ISSUE_FORMS=(
  .github/ISSUE_TEMPLATE/bug_report.yml
  .github/ISSUE_TEMPLATE/feature_request.yml
  .github/ISSUE_TEMPLATE/config.yml
)

ALLOWED_DIRS=(
  .github
  .github/ISSUE_TEMPLATE
)

usage() {
  sed -n '2,13p' "$0"
  exit 2
}

fail() {
  echo "check-public-tree: $*" >&2
  exit 1
}

is_allowed() {
  local name="$1" allowed
  for allowed in "${ALLOWED[@]}"; do
    [[ "$name" == "$allowed" ]] && return 0
  done
  for allowed in "${ALLOWED_ISSUE_FORMS[@]}"; do
    [[ "$name" == "$allowed" ]] && return 0
  done
  return 1
}

is_allowed_dir() {
  local name="$1" allowed
  for allowed in "${ALLOWED_DIRS[@]}"; do
    [[ "$name" == "$allowed" ]] && return 0
  done
  return 1
}

scan() {
  local root="$1" path rel
  [[ -d "$root" ]] || fail "not a directory: $root"

  while IFS= read -r -d '' path; do
    rel="${path#"$root"/}"
    case "$rel" in
      .git | .git/*) continue ;;
    esac
    if [[ -d "$path" ]]; then
      is_allowed_dir "$rel" || fail "directory in the public tree: $rel"
      continue
    fi
    is_allowed "$rel" || fail "path outside the allowed documents: $rel"
    case "$rel" in
      *.rs | *.ts | *.svelte | *.js | *.py | *.toml)
        fail "source file in the public tree: $rel"
        ;;
    esac
  done < <(find "$root" -mindepth 1 -print0)

  local hits
  hits=$(grep -RInE \
    'ghp_[A-Za-z0-9]{20,}|github_pat_[A-Za-z0-9_]{20,}|BEGIN [A-Z ]*PRIVATE KEY' \
    "$root" --exclude-dir=.git || true)
  [[ -z "$hits" ]] || fail "secret pattern found: $hits"

  hits=$(grep -RInE '(src-tauri|\.specify|\.claude)(/|[^A-Za-z0-9_-]|$)|(^|[^A-Za-z0-9_])specs/' \
    "$root" --exclude-dir=.git || true)
  [[ -z "$hits" ]] || fail "reference to an internal path found: $hits"
}

self_test() {
  local tmp
  tmp=$(mktemp -d)
  # shellcheck disable=SC2064
  trap "rm -rf '$tmp'" EXIT

  make_good() {
    local root="$1" name
    mkdir -p "$root/.github/ISSUE_TEMPLATE"
    for name in "${ALLOWED[@]}"; do
      printf '# %s\n\nVerso Translate — published document.\n' "$name" >"$root/$name"
    done
    for name in "${ALLOWED_ISSUE_FORMS[@]}"; do
      printf 'name: Form\n' >"$root/$name"
    done
  }

  expect_ok() {
    local name="$1" root="$2"
    if ! (scan "$root" >/dev/null 2>&1); then
      echo "self-test: expected success for $name" >&2
      return 1
    fi
  }

  expect_fail() {
    local name="$1" root="$2"
    if (scan "$root" >/dev/null 2>&1); then
      echo "self-test: expected failure for $name" >&2
      return 1
    fi
  }

  local root
  root="$tmp/good"
  make_good "$root"
  expect_ok "the allowed documents" "$root"
  mkdir -p "$root/.git"
  printf 'ref: refs/heads/main\n' >"$root/.git/HEAD"
  expect_ok "a .git directory" "$root"

  root="$tmp/source"
  make_good "$root"
  printf 'fn main() {}\n' >"$root/main.rs"
  expect_fail "a source file" "$root"

  root="$tmp/specs"
  make_good "$root"
  mkdir -p "$root/specs"
  printf 'spec\n' >"$root/specs/spec.md"
  expect_fail "a specs directory" "$root"

  root="$tmp/extra"
  make_good "$root"
  printf 'notes\n' >"$root/NOTES.md"
  expect_fail "an unexpected document" "$root"

  root="$tmp/secret"
  make_good "$root"
  printf 'token: ghp_000000000000000000000000\n' >>"$root/README.md"
  expect_fail "a secret pattern" "$root"

  root="$tmp/public"
  make_good "$root"
  printf 'Releases: github.com/example/verso-translate/releases/latest\n' >>"$root/README.md"
  expect_ok "a reference to the public repository" "$root"

  root="$tmp/internal"
  make_good "$root"
  printf 'See src-tauri/resources/languages.json for the table.\n' >>"$root/THIRD_PARTY.md"
  expect_fail "an internal path" "$root"

  root="$tmp/issues"
  make_good "$root"
  printf 'name: Extra\n' >"$root/.github/ISSUE_TEMPLATE/other.yml"
  expect_fail "an unexpected issue form" "$root"

  root="$tmp/workflows"
  make_good "$root"
  mkdir -p "$root/.github/workflows"
  printf 'name: CI\n' >"$root/.github/workflows/ci.yml"
  expect_fail "a workflow in the public tree" "$root"
}

case "${1:-}" in
  --self-test)
    self_test
    echo "check-public-tree: self-test passed"
    exit 0
    ;;
  -h | --help) usage ;;
esac

[[ $# -eq 1 ]] || usage
scan "$1"
echo "check-public-tree: $1 holds only the allowed documents"
