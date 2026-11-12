#!/usr/bin/env bash
# Writes `latest.json`, the manifest tauri-plugin-updater reads (007 T044/T047,
# contracts/update-channel.md). Stable releases only are ever offered: the endpoint is
# `releases/latest`, which ignores pre-releases anyway.
#
# Usage: scripts/make-update-manifest.sh <dist-dir> <tag> <owner/repo> [notes-file]
#        scripts/make-update-manifest.sh --self-test
#
# Inputs in <dist-dir> (the names the release build produces):
#   Verso_<version>_aarch64.app.tar.gz (+ .sig)   → darwin-aarch64
#   Verso_<version>_x64.app.tar.gz     (+ .sig)   → darwin-x86_64
#   Verso_<version>_x64-setup.exe      (+ .sig)   → windows-x86_64
set -euo pipefail

usage() {
  sed -n '2,14p' "$0"
  exit 2
}

fail() {
  echo "make-update-manifest: $*" >&2
  exit 1
}

manifest() {
  local dir="$1" tag="$2" repo="$3" notes_file="${4:-}"
  [[ -d "$dir" ]] || fail "not a directory: $dir"
  [[ "$tag" == v* ]] || fail "the tag must start with v (got '$tag')"

  local version="${tag#v}"
  local notes=""
  if [[ -n "$notes_file" ]]; then
    [[ -f "$notes_file" ]] || fail "notes file not found: $notes_file"
    notes=$(cat "$notes_file")
  fi
  local published
  published=$(LC_ALL=C date -u +%Y-%m-%dT%H:%M:%SZ)

  VERSION="$version" TAG="$tag" REPO="$repo" NOTES="$notes" PUBLISHED="$published" \
    DIST="$dir" python3 - <<'PY'
import json
import os
import sys

version = os.environ["VERSION"]
tag = os.environ["TAG"]
repo = os.environ["REPO"]
dist = os.environ["DIST"]

platforms = {
    "darwin-aarch64": f"Verso_{version}_aarch64.app.tar.gz",
    "darwin-x86_64": f"Verso_{version}_x64.app.tar.gz",
    "windows-x86_64": f"Verso_{version}_x64-setup.exe",
}

out = {}
for target, name in platforms.items():
    bundle = os.path.join(dist, name)
    if not os.path.isfile(bundle):
        sys.exit(f"make-update-manifest: missing updater bundle: {name}")
    sig_path = bundle + ".sig"
    if not os.path.isfile(sig_path):
        sys.exit(f"make-update-manifest: missing signature: {name}.sig")
    with open(sig_path, encoding="utf-8") as handle:
        signature = handle.read().strip()
    if not signature:
        sys.exit(f"make-update-manifest: empty signature: {name}.sig")
    out[target] = {
        "url": f"https://github.com/{repo}/releases/download/{tag}/{name}",
        "signature": signature,
    }

print(
    json.dumps(
        {
            "version": version,
            "notes": os.environ["NOTES"],
            "pub_date": os.environ["PUBLISHED"],
            "platforms": out,
        },
        indent=2,
    )
)
PY
}

self_test() {
  local tmp
  tmp=$(mktemp -d)
  # shellcheck disable=SC2064
  trap "rm -rf '$tmp'" EXIT
  local dist="$tmp/dist"
  mkdir -p "$dist"

  make_set() {
    local version="$1"
    printf 'bundle\n' >"$dist/Verso_${version}_aarch64.app.tar.gz"
    printf 'sig-a\n' >"$dist/Verso_${version}_aarch64.app.tar.gz.sig"
    printf 'bundle\n' >"$dist/Verso_${version}_x64.app.tar.gz"
    printf 'sig-b\n' >"$dist/Verso_${version}_x64.app.tar.gz.sig"
    printf 'installer\n' >"$dist/Verso_${version}_x64-setup.exe"
    printf 'sig-c\n' >"$dist/Verso_${version}_x64-setup.exe.sig"
  }

  make_set 0.1.0
  printf 'What shipped\n' >"$tmp/notes.md"
  local json
  json=$(manifest "$dist" v0.1.0 example/verso-translate "$tmp/notes.md") || {
    echo "self-test: a good set failed" >&2
    return 1
  }
  VERSION=0.1.0 TAG=v0.1.0 JSON="$json" python3 - <<'PY' || return 1
import json
import os
import sys

data = json.loads(os.environ["JSON"])
assert data["version"] == "0.1.0", data
assert data["notes"] == "What shipped", data
assert set(data["platforms"]) == {"darwin-aarch64", "darwin-x86_64", "windows-x86_64"}, data
expected = {
    "darwin-aarch64": "Verso_0.1.0_aarch64.app.tar.gz",
    "darwin-x86_64": "Verso_0.1.0_x64.app.tar.gz",
    "windows-x86_64": "Verso_0.1.0_x64-setup.exe",
}
for target, name in expected.items():
    platform = data["platforms"][target]
    assert platform["url"].endswith(f"/v0.1.0/{name}"), platform
    assert platform["signature"], platform
PY

  rm "$dist/Verso_0.1.0_x64.app.tar.gz.sig"
  if manifest "$dist" v0.1.0 example/verso-translate >/dev/null 2>&1; then
    echo "self-test: a missing signature was accepted" >&2
    return 1
  fi

  : >"$dist/Verso_0.1.0_x64.app.tar.gz.sig"
  if manifest "$dist" v0.1.0 example/verso-translate >/dev/null 2>&1; then
    echo "self-test: an empty signature was accepted" >&2
    return 1
  fi

  rm "$dist/Verso_0.1.0_x64-setup.exe"
  if manifest "$dist" v0.1.0 example/verso-translate >/dev/null 2>&1; then
    echo "self-test: a missing installer was accepted" >&2
    return 1
  fi
}

case "${1:-}" in
  --self-test)
    self_test
    echo "make-update-manifest: self-test passed"
    exit 0
    ;;
  -h | --help) usage ;;
esac

[[ $# -ge 3 && $# -le 4 ]] || usage
manifest "$1" "$2" "$3" "${4:-}"
