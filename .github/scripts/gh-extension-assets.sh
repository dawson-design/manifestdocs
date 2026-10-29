#!/bin/sh
# Turns dist's release archives into the bare binaries that
# `gh extension install dawson-design/gh-manifestdocs` downloads.
# gh picks the release asset whose name ends in its own OS-arch pair.
#
# Usage: gh-extension-assets.sh ARCHIVE_DIR OUT_DIR
set -eu

archives=$1
out=$2
mkdir -p "$out"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

gh_platform() {
  case $1 in
    aarch64-apple-darwin) echo darwin-arm64 ;;
    x86_64-unknown-linux-gnu) echo linux-amd64 ;;
    aarch64-unknown-linux-gnu) echo linux-arm64 ;;
    *) echo "no gh platform for target $1" >&2; return 1 ;;
  esac
}

count=0
for archive in "$archives"/manifestdocs-*.tar.xz; do
  [ -e "$archive" ] || break
  name=$(basename "$archive" .tar.xz)
  platform=$(gh_platform "${name#manifestdocs-}")
  tar -xJf "$archive" -C "$work"
  install -m 755 "$work/$name/manifestdocs" "$out/gh-manifestdocs-$platform"
  echo "$out/gh-manifestdocs-$platform"
  count=$((count + 1))
done

if [ "$count" -eq 0 ]; then
  echo "no manifestdocs-*.tar.xz archives in $archives" >&2
  exit 1
fi
