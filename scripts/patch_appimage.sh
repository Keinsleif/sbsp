#!/usr/bin/env bash
# Post-process the Tauri-built AppImage so it runs on distros newer than the
# CI build image. Tauri's default AppImage bundling has no exclude knob
# (tauri-apps/tauri#15665), so we repair the artifact after `tauri build`:
#
#   1. Strip bundled libwayland-*: libwayland must come from the host so it
#      matches the host's Mesa/EGL stack. Shipping the build machine's copy
#      makes eglGetDisplay() fail with EGL_BAD_PARAMETER on Mesa >= 25 hosts
#      (WebKitWebProcess aborts, window stays blank). See issue #128.
#   2. Neutralize the GST_PLUGIN_SYSTEM_PATH(_1_0) exports baked into the
#      AppRun.wrapped ELF. They point at usr/lib/gstreamer-1.0, which we never
#      populate (bundleMediaFramework is off), which disables GStreamer's
#      default plugin search and rebuilds the user's plugin registry cache as
#      empty. The rename keeps string lengths identical, so the ELF layout is
#      untouched.
#
# The caller must re-sign the AppImage afterwards (tauri signer sign): the
# updater signature covers the exact bytes and this script changes them.
set -euo pipefail

EXCLUDELIST_URL="https://raw.githubusercontent.com/AppImageCommunity/pkg2appimage/19e30b276ffedf4d3b4b56bc6320f463625a74f8/excludelist"
EXCLUDELIST_LOCAL="scripts/excludelist"

fetch_excludelist() {
  if [ -f "$EXCLUDELIST_LOCAL" ]; then
    cat "$EXCLUDELIST_LOCAL"
  else
    curl -fsSL "$EXCLUDELIST_URL"
  fi
}

# コメント行・空行を除いた「ライブラリ名のみ」の一覧を作る
mapfile -t EXCLUDE_NAMES < <(
  fetch_excludelist | sed 's/#.*//' | sed '/^[[:space:]]*$/d' | awk '{print $1}'
)

if [ $# -ne 1 ]; then
  echo "usage: $0 <path-to-AppImage>" >&2
  exit 1
fi

appimage="$(readlink -f "$1")"
test -f "$appimage"
test -x "$appimage"

# The squashfs payload starts right after the runtime ELF; the runtime locates
# it the same way (end of its own section header table), so a repack must keep
# this exact offset.
runtime_size="$(python3 - "$appimage" <<'PY'
import struct
import sys

with open(sys.argv[1], "rb") as f:
    header = f.read(64)
e_shoff = struct.unpack_from("<Q", header, 0x28)[0]
e_shentsize = struct.unpack_from("<H", header, 0x3A)[0]
e_shnum = struct.unpack_from("<H", header, 0x3C)[0]
print(e_shoff + e_shentsize * e_shnum)
PY
)"

read_magic() {
  dd if="$1" bs=1 skip="$runtime_size" count=4 2>/dev/null
}

if [ "$(read_magic "$appimage")" != "hsqs" ]; then
  echo "error: no squashfs superblock at offset $runtime_size of $appimage" >&2
  exit 1
fi

# Mirror the original superblock's compression and block size so the repacked
# payload stays readable by the embedded runtime even if Tauri changes them.
superblock="$(unsquashfs -s -o "$runtime_size" "$appimage")"
compression="$(printf '%s\n' "$superblock" | awk '/^Compression/ {print $2}')"
block_size="$(printf '%s\n' "$superblock" | awk '/^Block size/ {print $3}')"
test -n "$compression"
test -n "$block_size"

workdir="$(mktemp -d)"
trap 'rm -rf "$workdir"' EXIT
cd "$workdir"

"$appimage" --appimage-extract >/dev/null
appdir="$workdir/squashfs-root"
test -d "$appdir"

changed=0
removed=0
removed_list=()

# --- 変更: libwayland固定リストではなく、AppDir内の全ライブラリをexcludelistと突き合わせ ---
while IFS= read -r -d '' file; do
  base="$(basename "$file")"
  for pattern in "${EXCLUDE_NAMES[@]}"; do
    # excludelistはバージョン付きの厳密なファイル名（例: libwayland-client.so.0）
    # だが実際の同梱物は libwayland-client.so.0.x.y のように更に細かいことがあるため前方一致で見る
    if [[ "$base" == "$pattern"* ]]; then
      removed_list+=("$file")
      break
    fi
  done
done < <(find "$appdir/usr/lib" -type f -print0 2>/dev/null)

if [ "${#removed_list[@]}" -gt 0 ]; then
  printf 'stripping (in excludelist): %s\n' "${removed_list[@]#"$appdir"/}"
  rm -f -- "${removed_list[@]}"
  removed="${#removed_list[@]}"
  changed=1
fi

apprun_wrapped="$appdir/AppRun.wrapped"
if [ -f "$apprun_wrapped" ] && grep -aq 'GST_PLUGIN_SYSTEM_PATH' "$apprun_wrapped"; then
  size_before="$(wc -c <"$apprun_wrapped")"
  perl -pi -e 's/GST_PLUGIN_SYSTEM_PATH/_ST_PLUGIN_SYSTEM_PATH/g' "$apprun_wrapped"
  size_after="$(wc -c <"$apprun_wrapped")"
  if [ "$size_before" != "$size_after" ]; then
    echo "error: AppRun.wrapped size changed ($size_before -> $size_after)" >&2
    exit 1
  fi
  echo "neutralized GST_PLUGIN_SYSTEM_PATH exports in AppRun.wrapped"
  changed=1
fi

if [ "$changed" -eq 0 ]; then
  echo "nothing to repair (upstream bundling fixed?); leaving AppImage untouched"
  exit 0
fi

head -c "$runtime_size" "$appimage" >runtime.bin
mksquashfs "$appdir" filesystem.squashfs \
  -comp "$compression" -b "$block_size" -noappend -all-root -mkfs-time 0 -quiet
cat runtime.bin filesystem.squashfs >repacked.AppImage
chmod 755 repacked.AppImage

if [ "$(read_magic repacked.AppImage)" != "hsqs" ]; then
  echo "error: repacked image lost the squashfs offset" >&2
  exit 1
fi

list_entries() {
  unsquashfs -o "$runtime_size" -l "$1" | grep -c '^squashfs-root'
}

orig_entries="$(list_entries "$appimage")"
new_entries="$(list_entries repacked.AppImage)"
if [ "$new_entries" -ne $((orig_entries - removed)) ]; then
  echo "error: repacked entry count $new_entries != $orig_entries - $removed" >&2
  exit 1
fi

mv repacked.AppImage "$appimage"
echo "repaired $appimage ($orig_entries -> $new_entries entries)"

cargo tauri signer sign "$appimage"

new_signature="$(cat "${appimage}.sig")"

jq --arg sig "$new_signature" \
  '.platforms["linux-x86_64"].signature = $sig' \
  latest.json > latest.json.new
mv latest.json.new latest.json
