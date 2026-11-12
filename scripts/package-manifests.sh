#!/usr/bin/env bash
# Generates the package-manager manifests from `SHA256SUMS.txt` (007 T060/T062,
# contracts/package-managers.md): the Homebrew cask and the winget trio.
# Pre-release tags produce nothing, so an rc never reaches a package manager.
#
# Usage: scripts/package-manifests.sh <dist-dir> <tag> <out-dir> [owner/repo]
#        scripts/package-manifests.sh --self-test
set -euo pipefail

usage() {
  sed -n '2,8p' "$0"
  exit 2
}

fail() {
  echo "package-manifests: $*" >&2
  exit 1
}

manifests() {
  local dir="$1" tag="$2" out="$3" repo="${4:-${PUBLIC_REPO:-}}"
  [[ -d "$dir" ]] || fail "not a directory: $dir"
  [[ "$tag" == v* ]] || fail "the tag must start with v (got '$tag')"
  [[ -n "$repo" ]] || fail "pass owner/repo or set PUBLIC_REPO"

  local version="${tag#v}"
  if [[ "$version" == *-* ]]; then
    echo "package-manifests: $tag is a pre-release; nothing to write"
    return 0
  fi
  [[ -f "$dir/SHA256SUMS.txt" ]] || fail "no SHA256SUMS.txt in $dir"
  mkdir -p "$out"

  DIST="$dir" TAG="$tag" VERSION="$version" REPO="$repo" OUT="$out" python3 - <<'PY'
import os
import sys
from pathlib import Path

dist = Path(os.environ["DIST"])
out = Path(os.environ["OUT"])
version = os.environ["VERSION"]
tag = os.environ["TAG"]
repo = os.environ["REPO"]
owner = repo.split("/")[0]
release = f"https://github.com/{repo}/releases/download/{tag}"

sums = {}
for line in (dist / "SHA256SUMS.txt").read_text(encoding="utf-8").splitlines():
    line = line.strip()
    if not line:
        continue
    parts = line.split()
    if len(parts) < 2:
        sys.exit(f"package-manifests: malformed checksum line: {line!r}")
    sums[parts[-1].lstrip("./")] = parts[0]

arm_name = f"Verso_{version}_aarch64.dmg"
intel_name = f"Verso_{version}_x64.dmg"
setup_name = f"Verso_{version}_x64-setup.exe"
for name in (arm_name, intel_name, setup_name):
    if name not in sums:
        sys.exit(f"package-manifests: SHA256SUMS.txt has no entry for {name}")

arm_sha, intel_sha, setup_sha = sums[arm_name], sums[intel_name], sums[setup_name]

cask = f'''cask "verso" do
  version "{version}"

  on_arm do
    sha256 "{arm_sha}"
    url "{release}/{arm_name}"
  end
  on_intel do
    sha256 "{intel_sha}"
    url "{release}/{intel_name}"
  end

  name "Verso"
  desc "Instant translation of selected or typed text"
  homepage "https://github.com/{repo}"
  app "Verso.app"

  auto_updates true
  depends_on macos: ">= :ventura"

  postflight do
    target = "#{{appdir}}/Verso.app"
    system_command "/usr/bin/xattr", args: ["-dr", "com.apple.quarantine", target] if File.exist?(target)
  end
end
'''
(out / "verso.rb").write_text(cask, encoding="utf-8")

identifier = f"{owner}.Verso"
manifest_version = "1.6.0"
(out / f"{identifier}.yaml").write_text(
    f"""# yaml-language-server: $schema=https://aka.ms/winget-manifest.version.{manifest_version}.schema.json
PackageIdentifier: {identifier}
PackageVersion: {version}
DefaultLocale: en-US
ManifestType: version
ManifestVersion: {manifest_version}
""",
    encoding="utf-8",
)

(out / f"{identifier}.locale.en-US.yaml").write_text(
    f"""# yaml-language-server: $schema=https://aka.ms/winget-manifest.defaultLocale.{manifest_version}.schema.json
PackageIdentifier: {identifier}
PackageVersion: {version}
PackageLocale: en-US
Publisher: Verso Translate
PublisherUrl: https://github.com/{repo}
PackageName: Verso
PackageUrl: https://github.com/{repo}
License: Proprietary
LicenseUrl: https://github.com/{repo}/blob/main/TERMS.md
ShortDescription: Instant translation of selected or typed text
Description: A lightweight menu bar translator for selected or typed text, with Google and Bing engines and automatic fallback.
Moniker: verso
Tags:
  - translation
  - translator
  - tray
  - menu-bar
ManifestType: defaultLocale
ManifestVersion: {manifest_version}
""",
    encoding="utf-8",
)

(out / f"{identifier}.installer.yaml").write_text(
    f"""# yaml-language-server: $schema=https://aka.ms/winget-manifest.installer.{manifest_version}.schema.json
PackageIdentifier: {identifier}
PackageVersion: {version}
InstallerType: nullsoft
InstallModes:
  - interactive
  - silent
InstallerSwitches:
  Silent: /S
Installers:
  - Architecture: x64
    InstallerUrl: {release}/{setup_name}
    InstallerSha256: {setup_sha}
ManifestType: installer
ManifestVersion: {manifest_version}
""",
    encoding="utf-8",
)
PY
}

self_test() {
  local tmp
  tmp=$(mktemp -d)
  # shellcheck disable=SC2064
  trap "rm -rf '$tmp'" EXIT
  local dist="$tmp/dist" out="$tmp/out"
  mkdir -p "$dist"

  printf 'installer\n' >"$dist/Verso_0.1.0_aarch64.dmg"
  printf 'installer\n' >"$dist/Verso_0.1.0_x64.dmg"
  printf 'installer\n' >"$dist/Verso_0.1.0_x64-setup.exe"
  {
    printf 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa  Verso_0.1.0_aarch64.dmg\n'
    printf 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb  Verso_0.1.0_x64.dmg\n'
    printf 'cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc  Verso_0.1.0_x64-setup.exe\n'
  } >"$dist/SHA256SUMS.txt"

  manifests "$dist" v0.1.0 "$out" example/verso-translate
  local file
  for file in verso.rb example.Verso.yaml example.Verso.locale.en-US.yaml example.Verso.installer.yaml; do
    [[ -f "$out/$file" ]] || {
      echo "self-test: $file was not written" >&2
      return 1
    }
  done
  grep -q 'sha256 "aaaaaaaa' "$out/verso.rb" || {
    echo "self-test: the cask has no arm checksum" >&2
    return 1
  }
  grep -q 'com.apple.quarantine' "$out/verso.rb" || {
    echo "self-test: the cask does not clear the quarantine flag" >&2
    return 1
  }
  grep -q 'cccccccccccccccc' "$out/example.Verso.installer.yaml" || {
    echo "self-test: the winget installer has no checksum" >&2
    return 1
  }
  grep -q 'InstallerType: nullsoft' "$out/example.Verso.installer.yaml" || {
    echo "self-test: the winget installer is not nullsoft" >&2
    return 1
  }

  local pre="$tmp/pre"
  if ! manifests "$dist" v0.2.0-rc.1 "$pre" example/verso-translate >/dev/null; then
    echo "self-test: a pre-release tag failed instead of writing nothing" >&2
    return 1
  fi
  if [[ -d "$pre" ]] && [[ -n "$(ls -A "$pre" 2>/dev/null)" ]]; then
    echo "self-test: a pre-release tag wrote manifests" >&2
    return 1
  fi

  rm "$dist/SHA256SUMS.txt"
  if ( manifests "$dist" v0.1.0 "$tmp/missing" example/verso-translate ) >/dev/null 2>&1; then
    echo "self-test: a missing SHA256SUMS.txt was accepted" >&2
    return 1
  fi
}

case "${1:-}" in
  --self-test)
    self_test
    echo "package-manifests: self-test passed"
    exit 0
    ;;
  -h | --help) usage ;;
esac

[[ $# -ge 3 && $# -le 4 ]] || usage
manifests "$1" "$2" "$3" "${4:-}"
