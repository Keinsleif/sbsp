#!/usr/bin/env python3
# SPDX-License-Identifier: Elastic-2.0
# Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)
"""Builds the Velopack package of one app from its release binary.

    python scripts/pack.py host|remote [extra vpk arguments]

Build the binary first (`mise run build_host` / `build_remote`). The package
lands in target/dist/<app>/.

Environment:
  SBSP_BIN_DIR   where the release binary is (default target/release, and
                 target/universal on macOS: see scripts/build.py)
  SBSP_FEED_URL  the published feed of this app: when set, the previous
                 release is downloaded first so that delta packages are made

Signing (nothing is signed unless these are set):
  SBSP_WIN_SIGN_PARAMS    signtool parameters (vpk --signParams)
  SBSP_WIN_SIGN_TEMPLATE  a signing command with {{file}} (vpk --signTemplate),
                          for cloud signing tools
  SBSP_MAC_SIGN_APP_IDENTITY      "Developer ID Application: ..."
  SBSP_MAC_SIGN_INSTALL_IDENTITY  "Developer ID Installer: ..."
  SBSP_MAC_NOTARY_PROFILE         a notarytool keychain profile
"""

import os
import re
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

APPS = {
    "host": ("com.keinsleif.sbsp", "sbsp_host", "SBS Player"),
    "remote": ("com.keinsleif.sbsp.remote", "sbsp_remote", "SBS Player Remote"),
}

# environment variable -> vpk option
SIGNING = {
    "win32": {
        "SBSP_WIN_SIGN_PARAMS": "--signParams",
        "SBSP_WIN_SIGN_TEMPLATE": "--signTemplate",
    },
    "darwin": {
        "SBSP_MAC_SIGN_APP_IDENTITY": "--signAppIdentity",
        "SBSP_MAC_SIGN_INSTALL_IDENTITY": "--signInstallIdentity",
        "SBSP_MAC_NOTARY_PROFILE": "--notaryProfile",
    },
}


def version() -> str:
    """The version of sbsp_ui (the apps are released together)."""
    manifest = (ROOT / "sbsp_ui" / "Cargo.toml").read_text(encoding="utf-8")
    match = re.search(r'^version\s*=\s*"([^"]+)"', manifest, re.MULTILINE)
    if not match:
        sys.exit("no version in sbsp_ui/Cargo.toml")
    return match.group(1)


def signing_options() -> list[str]:
    options: list[str] = []
    for variable, option in SIGNING.get(sys.platform, {}).items():
        value = os.environ.get(variable)
        if value:
            options += [option, value]
    return options


def main() -> None:
    if len(sys.argv) < 2 or sys.argv[1] not in APPS:
        sys.exit("usage: pack.py host|remote [vpk arguments]")
    app, extra = sys.argv[1], sys.argv[2:]
    pack_id, binary, title = APPS[app]

    windows = sys.platform == "win32"
    exe = binary + (".exe" if windows else "")
    icon = {
        "win32": "icon.ico",
        "darwin": "icon.icns",
    }.get(sys.platform, "128x128.png")

    stage = ROOT / "target" / "pack" / app
    out = ROOT / "target" / "dist" / app
    shutil.rmtree(stage, ignore_errors=True)
    stage.mkdir(parents=True)
    out.mkdir(parents=True, exist_ok=True)

    default_dir = "universal" if sys.platform == "darwin" else "release"
    bin_dir = Path(os.environ.get("SBSP_BIN_DIR", ROOT / "target" / default_dir))
    shutil.copy2(bin_dir / exe, stage / exe)
    shutil.copy2(ROOT / "THIRD_PARTY_NOTICES.md", stage)

    vpk = shutil.which("vpk")
    if not vpk:
        sys.exit("vpk not found: dotnet tool install -g vpk (needs the .NET 8 SDK)")

    feed = os.environ.get("SBSP_FEED_URL")
    if feed:
        previous = subprocess.run([vpk, "download", "http", "--url", feed, "--outputDir", str(out)])
        if previous.returncode != 0:
            print("no previous release; making a full package only")

    command = [
        vpk, "pack",
        "--packId", pack_id,
        "--packVersion", version(),
        "--packDir", str(stage),
        "--mainExe", exe,
        "--packTitle", title,
        "--packAuthors", "Keinsleif",
        "--icon", str(ROOT / "sbsp_ui" / "assets" / "icons" / icon),
        "--outputDir", str(out),
        *signing_options(),
        *extra,
    ]
    sys.exit(subprocess.run(command).returncode)


if __name__ == "__main__":
    main()
