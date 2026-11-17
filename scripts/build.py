#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)
"""Builds the release binary of one app.

    python scripts/build.py host|remote

The binary is target/release/<bin>. On macOS it is built for both
architectures and joined (target/universal/<bin>), which is what
scripts/pack.py packs there. The host embeds the web build (sbsp_ui/web_dist):
build that first (`mise run build_web`).
"""

import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BINARIES = {"host": "sbsp_host", "remote": "sbsp_remote"}
TRIPLES = ["aarch64-apple-darwin", "x86_64-apple-darwin"]


def cargo(app: str, *extra: str) -> None:
    subprocess.run(
        [
            "cargo", "build", "-p", "sbsp_ui", "--profile", "release",
            "--no-default-features", "--features", app,
            "--bin", BINARIES[app], *extra,
        ],
        check=True,
        cwd=ROOT,
    )


def main() -> None:
    if len(sys.argv) != 2 or sys.argv[1] not in BINARIES:
        sys.exit("usage: build.py host|remote")
    app = sys.argv[1]
    binary = BINARIES[app]

    if sys.platform != "darwin":
        cargo(app)
        return

    for triple in TRIPLES:
        subprocess.run(["rustup", "target", "add", triple], check=True)
        cargo(app, "--target", triple)
    out = ROOT / "target" / "universal"
    out.mkdir(parents=True, exist_ok=True)
    subprocess.run(
        [
            "lipo", "-create", "-output", str(out / binary),
            *(str(ROOT / "target" / triple / "release" / binary) for triple in TRIPLES),
        ],
        check=True,
    )


if __name__ == "__main__":
    main()
