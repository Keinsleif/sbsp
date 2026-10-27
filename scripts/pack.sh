#!/usr/bin/env bash
# Builds the Velopack package of one app from its release binary.
#
#   scripts/pack.sh host|remote [extra vpk arguments]
#
# Build the binary first (`mise run build_host` / `build_remote`). The
# package lands in target/releases/<app>/.
#
# SBSP_BIN_DIR   where the release binary is (default target/release; CI
#                points it at the universal binary on macOS)
# SBSP_FEED_URL  the published feed of this app: when set, the previous
#                release is downloaded first so that delta packages are made
set -euo pipefail

app=${1:?usage: pack.sh host|remote [vpk args]}
shift
cd "$(dirname "$0")/.."

case "$app" in
  host) id=com.keinsleif.sbsp; bin=sbsp_host; title="SBS Player" ;;
  remote) id=com.keinsleif.sbsp.remote; bin=sbsp_remote; title="SBS Player Remote" ;;
  *) echo "unknown app: $app" >&2; exit 2 ;;
esac

version=$(cargo pkgid -p sbsp_ui | sed 's/.*[#@]//')
exe=$bin
icon=sbsp_ui/assets/icons/128x128.png
case "$(uname -s)" in
  Darwin) icon=sbsp_ui/assets/icons/icon.icns ;;
  MINGW*|MSYS*|CYGWIN*) exe=$bin.exe; icon=sbsp_ui/assets/icons/icon.ico ;;
esac

stage=target/pack/$app
out=target/releases/$app
rm -rf "$stage"
mkdir -p "$stage" "$out"
cp "${SBSP_BIN_DIR:-target/release}/$exe" "$stage/"
cp THIRD_PARTY_NOTICES.md "$stage/"

if [ -n "${SBSP_FEED_URL:-}" ]; then
  vpk download http --url "$SBSP_FEED_URL" --outputDir "$out" || echo "no previous release; making a full package only"
fi

vpk pack \
  --packId "$id" \
  --packVersion "$version" \
  --packDir "$stage" \
  --mainExe "$exe" \
  --packTitle "$title" \
  --packAuthors Keinsleif \
  --icon "$icon" \
  --outputDir "$out" \
  "$@"
