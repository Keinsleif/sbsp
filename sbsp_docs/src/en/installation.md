---
title: Installation
description: How to get SBS Player
---

# Installation

## Download

You can obtain the installer or the application itself from the following source.

<DownloadButton />

This software operates in a local environment and does not collect, transmit, or store any personal information or usage data.

> If you are using v2 and wish to update to v3, please refer to [Migrating from v2 to v3](./migration/v2tov3).

Alternatively, you can obtain the installer from the following page.

::: details GitHub Releases
[GitHub Releases](https://github.com/Keinsleif/sbsp/releases)

Download the file corresponding to your operating system from the "Assets" section on the page mentioned above.

For Windows, download `SBSP-Host-Windows-Setup.exe` (or `SBSP-Host-Windows-Portable.zip` if you prefer a version that does not require installation).

For macOS, download `SBSP-Host-macOS-Setup.pkg` (or `SBSP-Host-macOS-Portable.zip`).

For Linux, download `SBSP-Host-Linux.AppImage`.
:::

## If a security warning appears on Windows

The current distribution for Windows is not digitally signed. Consequently, when you launch the installer, you may see a message stating, "Windows protected your PC."
You can proceed by clicking "More info" and then selecting the "Run" button that appears.

> Verifying the hash value of the downloaded file is strongly recommended before following the steps above.
> For instructions on how to perform this verification, please refer to [How to verify hash values](#how-to-verify-hash-values).

## If you cannot open the application on macOS

The current macOS distribution has not been signed or notarized by Apple.
Consequently, if you attempt to open the downloaded file directly, you may encounter a warning—such as "cannot verify the developer"—that prevents the application from launching. Please use one of the following methods to open it.

> Verifying the hash value of the downloaded file is strongly recommended before following the steps below.
> For instructions on how to perform this verification, please refer to [How to verify hash values](#how-to-verify-hash-values).

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

## How to Verify Hash Values

1. Download the file
    Download the latest file using the instructions provided at the top of the page.

2. Calculate the hash value

    ::: details For macOS
    1. Open Terminal.
    2. Run the following command:

    ```zsh
    shasum -a 256 <file_path>
    ```

    You can specify the `<file_path>` by dragging and dropping the file into the Terminal window.
    :::

    ::: details For Windows
    1. Open a terminal (Windows Terminal).
    2. Launch Windows PowerShell:

        ```cmd
        powershell
        ```

        Type the command and press Enter.
    3. Run the following command:

        ```powershell
        Get-FileHash <file_path>
        ```

        You can specify the `<file_path>` by dragging and dropping the file into the terminal window.
    :::

3. Verify the hash value
    On the [GitHub Releases](https://github.com/Keinsleif/sbsp/releases/latest) page, the downloaded file and its corresponding hash value are listed under "Assets."
    Example: `sha256:9d0775984bc4341f61388c93d...`

    Copy the hash value (using the copy icon) and compare it with the value you calculated in the previous step to ensure they match.

<script setup>
    import DownloadButton from '/components/DownloadButton.vue';
</script>
