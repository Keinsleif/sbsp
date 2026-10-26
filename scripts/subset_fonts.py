#!/usr/bin/env python3
# SPDX-License-Identifier: Elastic-2.0
# Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)
"""Makes the web build's fonts: the UI fonts cut down to what a browser needs.

The desktop builds embed the whole BIZ UD fonts (about 4.6 MB each). The
web build fetches these instead (see `sbsp_ui/src/web.rs`):

  web/fonts/ui.ttf    BIZ UDPGothic: ASCII and all of JIS X 0208 (kana,
                      symbols, 6355 kanji), which is what a name typed in
                      Japanese is made of.
  web/fonts/mono.ttf  BIZ UDGothic: ASCII and JIS X 0208's non-kanji rows
                      (the clock and the times; the credits' text).

The family names are kept (the fonts' license has no Reserved Font Name),
so the app asks for them the same way on every platform.

Needs `pyftsubset` (fonttools). Run from anywhere:
  python3 scripts/subset_fonts.py
"""
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
FONTS = ROOT / "sbsp_ui" / "assets" / "fonts"
OUT = ROOT / "sbsp_ui" / "web" / "fonts"
I18N = ROOT / "sbsp_ui" / "i18n"


def jis0208(rows):
    chars = []
    for row in rows:
        for cell in range(1, 95):
            try:
                chars.append(bytes([0xA0 + row, 0xA0 + cell]).decode("euc_jp"))
            except UnicodeDecodeError:
                pass
    return chars


ASCII = [chr(c) for c in range(0x20, 0x7F)]
SYMBOLS = jis0208(range(1, 9))
KANJI = jis0208(range(16, 85))


def subset(source, target, chars):
    OUT.mkdir(parents=True, exist_ok=True)
    text = OUT / (target + ".txt")
    text.write_text("".join(chars), encoding="utf-8")
    made = OUT / (target + ".new")
    command = [
        "pyftsubset",
        str(FONTS / source),
        f"--text-file={text}",
        "--layout-features=",
        "--no-hinting",
        f"--output-file={made}",
    ]
    subprocess.run(command, check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    text.unlink()
    # Replaced only when it differs: the file watcher of `trunk serve` takes
    # every write for a change and builds again, which runs this again.
    final = OUT / target
    if final.exists() and final.read_bytes() == made.read_bytes():
        made.unlink()
    else:
        made.replace(final)
    print(f"{target}: {final.stat().st_size:,} bytes")


def covered(path):
    from fontTools.ttLib import TTFont

    return set(TTFont(path).getBestCmap())


def main():
    for name in ("BIZUDPGothic-Regular.ttf", "BIZUDGothic-Regular.ttf"):
        if not (FONTS / name).exists():
            sys.exit(f"missing {FONTS / name} (see assets/fonts/README.md)")

    subset("BIZUDPGothic-Regular.ttf", "ui.ttf", ASCII + SYMBOLS + KANJI)
    subset("BIZUDGothic-Regular.ttf", "mono.ttf", ASCII + SYMBOLS)

    # What the app's own texts need must be in the UI font.
    have = covered(OUT / "ui.ttf")
    used = set()
    for ftl in I18N.glob("*/*.ftl"):
        used |= {ord(c) for c in ftl.read_text(encoding="utf-8") if not c.isspace()}
    missing = sorted(c for c in used if c not in have and c >= 0x20)
    if missing:
        print("not in the web UI font:", "".join(chr(c) for c in missing))


if __name__ == "__main__":
    main()
