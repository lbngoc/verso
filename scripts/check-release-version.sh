#!/usr/bin/env bash
# Guards a release tag against the three manifests and the public changelog (007 T003, FR-011).
#
# Usage: scripts/check-release-version.sh <tag> [root]
#        scripts/check-release-version.sh --self-test
#
# <tag> is vMAJOR.MINOR.PATCH or vMAJOR.MINOR.PATCH-suffix; a missing leading "v" is accepted.
# The suffix marks a pre-release (GitHub) but is not part of the app version: v0.1.0-rc.1 is checked
# against version 0.1.0 in:
#   src-tauri/tauri.conf.json  "version"
#   package.json               "version"
#   src-tauri/Cargo.toml       [package] version
# and docs/public/CHANGELOG.md must have a "## 0.1.0" section.
set -euo pipefail

usage() {
  sed -n '2,12p' "$0"
  exit 2
}

fail() {
  echo "check-release-version: $*" >&2
  exit 1
}

read_json_version() {
  sed -n 's/.*"version"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$1" | head -n 1
}

read_cargo_version() {
  sed -n 's/^version[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' "$1" | head -n 1
}

check_version() {
  local tag="$1" root="$2" full="${1#v}"
  local core="${full%%-*}"
  local semver='^[0-9]+\.[0-9]+\.[0-9]+([.-][0-9A-Za-z.-]+)?$'
  [[ "$full" =~ $semver ]] || fail "tag '$tag' is not vMAJOR.MINOR.PATCH[-suffix]"

  local tauri pkg cargo changelog
  tauri=$(read_json_version "$root/src-tauri/tauri.conf.json") ||
    fail "cannot read src-tauri/tauri.conf.json"
  pkg=$(read_json_version "$root/package.json") || fail "cannot read package.json"
  cargo=$(read_cargo_version "$root/src-tauri/Cargo.toml") || fail "cannot read src-tauri/Cargo.toml"
  changelog="$root/docs/public/CHANGELOG.md"

  [[ -n "$tauri" ]] || fail "no version in src-tauri/tauri.conf.json"
  [[ -n "$pkg" ]] || fail "no version in package.json"
  [[ -n "$cargo" ]] || fail "no version in src-tauri/Cargo.toml"
  [[ -f "$changelog" ]] || fail "missing $changelog"

  [[ "$tauri" == "$core" ]] ||
    fail "tag $tag does not match tauri.conf.json version $tauri"
  [[ "$pkg" == "$core" ]] ||
    fail "tag $tag does not match package.json version $pkg"
  [[ "$cargo" == "$core" ]] ||
    fail "tag $tag does not match Cargo.toml version $cargo"

  local escaped
  escaped=$(printf '%s' "$core" | sed 's/[][\.^$*+?(){}|]/\\&/g')
  grep -qE "^##[[:space:]]+${escaped}([[:space:]]|$)" "$changelog" ||
    fail "docs/public/CHANGELOG.md has no '## $core' section"
}

self_test() {
  local tmp
  tmp=$(mktemp -d)
  # shellcheck disable=SC2064
  trap "rm -rf '$tmp'" EXIT

  make_tree() {
    local root="$1" tauri_version="$2" pkg_version="$3" cargo_version="$4" changelog="$5"
    mkdir -p "$root/src-tauri" "$root/docs/public"
    printf '{\n  "productName": "Verso",\n  "version": "%s"\n}\n' "$tauri_version" \
      >"$root/src-tauri/tauri.conf.json"
    printf '{\n  "name": "verso-translate",\n  "version": "%s"\n}\n' "$pkg_version" \
      >"$root/package.json"
    printf '[package]\nname = "verso"\nversion = "%s"\n' "$cargo_version" \
      >"$root/src-tauri/Cargo.toml"
    if [[ "$changelog" == "yes" ]]; then
      printf '# Changelog\n\n## %s\n\n- entry\n\n## 0.0.9\n\n- old\n' "$tauri_version" \
        >"$root/docs/public/CHANGELOG.md"
    else
      printf '# Changelog\n\n## 0.0.9\n\n- old\n' >"$root/docs/public/CHANGELOG.md"
    fi
  }

  # check_version exits on a mismatch, so each probe runs in its own subshell.
  expect_ok() {
    local name="$1" tag="$2" root="$3"
    if ! (check_version "$tag" "$root" >/dev/null 2>&1); then
      echo "self-test: expected success for $name" >&2
      return 1
    fi
  }

  expect_fail() {
    local name="$1" tag="$2" root="$3"
    if (check_version "$tag" "$root" >/dev/null 2>&1); then
      echo "self-test: expected failure for $name" >&2
      return 1
    fi
  }

  local root
  root="$tmp/good"
  make_tree "$root" "0.1.0" "0.1.0" "0.1.0" yes
  expect_ok "matching tag" "v0.1.0" "$root"
  expect_ok "matching tag without v" "0.1.0" "$root"
  expect_fail "wrong tag" "v0.2.0" "$root"
  expect_fail "bad tag shape" "v0.1" "$root"

  make_tree "$tmp/tauri" "0.2.0" "0.1.0" "0.1.0" yes
  expect_fail "tauri.conf.json mismatch" "v0.1.0" "$tmp/tauri"

  make_tree "$tmp/pkg" "0.1.0" "0.2.0" "0.1.0" yes
  expect_fail "package.json mismatch" "v0.1.0" "$tmp/pkg"

  make_tree "$tmp/cargo" "0.1.0" "0.1.0" "0.2.0" yes
  expect_fail "Cargo.toml mismatch" "v0.1.0" "$tmp/cargo"

  make_tree "$tmp/notes" "0.1.0" "0.1.0" "0.1.0" no
  expect_fail "missing changelog section" "v0.1.0" "$tmp/notes"

  root="$tmp/rc"
  make_tree "$root" "0.1.0" "0.1.0" "0.1.0" yes
  expect_ok "pre-release tag" "v0.1.0-rc.1" "$root"
}

case "${1:-}" in
  --self-test)
    self_test
    echo "check-release-version: self-test passed"
    exit 0
    ;;
  -h | --help) usage ;;
esac

[[ $# -ge 1 ]] || usage
root="${2:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
version="${1#v}"
check_version "$1" "$root"
echo "check-release-version: $1 matches the manifests and the changelog section ${version%%-*}"
