---
title: Installation
description: How to get SBS Player
---

# Installation

## Download

You can obtain the installer or the application itself from the following source.

<DownloadButton />

Free code signing provided by SignPath.io, certificate by SignPath Foundation
This software operates in a local environment and does not collect, transmit, or store any personal information or usage data.

> If you are using v2 and wish to update to v3, please refer to [Migrating from v2 to v3](./migration/v2tov3).

Alternatively, you can obtain the installer from the following page.

### GitHub Releases

[GitHub Releases](https://github.com/Keinsleif/sbsp/releases)

Download the file corresponding to your operating system from the "Assets" section on the page mentioned above.

For Windows, download `SBSP-Host-Windows-Setup.exe` (or `SBSP-Host-Windows-Portable.zip` if you prefer a version that does not require installation).

For macOS, download `SBSP-Host-macOS-Setup.pkg` (or `SBSP-Host-macOS-Portable.zip`).

For Linux, download `SBSP-Host-Linux.AppImage`.

## If you cannot open the application on macOS

The current macOS distribution has not been signed or notarized by Apple.
Consequently, if you attempt to open the downloaded file directly, you may encounter a warning—such as "cannot verify the developer"—that prevents the application from launching. Please use one of the following methods to open it.

### Opening the `.pkg` installer

1. Double-click `SBSP-Host-macOS-Setup.pkg` and close any warning messages that appear.
2. Go to **System Settings** > **Privacy & Security**. Scroll down and click **Open Anyway** next to the message stating that "SBSP-Host-macOS-Setup.pkg" was blocked from opening.
3. After authentication, the installer will launch; follow the on-screen instructions to complete the installation.

On macOS 14 or earlier, you can also right-click (or hold the Control key while clicking) the `.pkg` file, select **Open**, and then click **Open** again in the confirmation dialog.

### Using the application distributed as a ZIP file

1. Extract `SBSP-Host-macOS-Portable.zip` and move the resulting application to your **Applications** folder.
2. Double-click the application and close any warning messages that appear.
3. Go to **System Settings** > **Privacy & Security**. Scroll down and click **Open Anyway** next to the message stating that "[App Name]" was blocked from opening.
4. After authentication, the application will launch.

This procedure is only required for the initial installation.

Note that these steps will no longer be necessary once a signed and notarized version of the application is made available.

After installation, you can check for and install new versions by selecting **Check for Updates** under the **Help** menu.

<script setup>
    import DownloadButton from '/components/DownloadButton.vue';
</script>
