// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! Taking over from the Tauri version on Windows.
//!
//! The Tauri version was installed by an NSIS installer, which lives next
//! to Velopack's installation instead of being replaced by it. On the
//! first run after a Velopack install (`VELOPACK_FIRSTRUN` is set), the old
//! version's uninstaller is run silently, so that only one copy is left.
//! The settings are not touched: the Tauri and the iced versions keep them
//! in the same place (the app identifier is the same).
//!
//! Nothing happens on other platforms (their old installs are replaced by
//! the user, and the Tauri version there never shared a location).

/// Removes the Tauri installation of `product_name` ("SBS Player" for the
/// host, "SBS Player Remote" for the remote), if there is one.
pub fn remove_tauri_install(product_name: &str) {
    if std::env::var_os("VELOPACK_FIRSTRUN").is_none() {
        return;
    }
    #[cfg(windows)]
    windows::remove(product_name);
    #[cfg(not(windows))]
    let _ = product_name;
}

/// The program in the `UninstallString` value of `reg query` output:
/// `    UninstallString    REG_SZ    "C:\...\uninstall.exe"`.
#[cfg_attr(not(any(windows, test)), allow(dead_code))]
fn uninstaller_in(output: &str) -> Option<String> {
    let line = output.lines().find(|line| line.contains("UninstallString"))?;
    let value = line.split("REG_SZ").nth(1)?.trim();
    let program = match value.strip_prefix('"') {
        Some(rest) => rest.split('"').next()?,
        None => value.split(" /").next()?.trim(),
    };
    let lower = program.to_ascii_lowercase();
    // Only an NSIS uninstaller is run: anything else is not ours to start.
    (lower.ends_with("uninstall.exe") || lower.ends_with("uninstall")).then(|| program.to_owned())
}

#[cfg(windows)]
mod windows {
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    use super::uninstaller_in;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    pub fn remove(product_name: &str) {
        let key = format!(
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\{product_name}"
        );
        let output = Command::new("reg")
            .args(["query", &key, "/v", "UninstallString"])
            .creation_flags(CREATE_NO_WINDOW)
            .output();
        let Some(program) = output
            .ok()
            .filter(|output| output.status.success())
            .and_then(|output| uninstaller_in(&String::from_utf8_lossy(&output.stdout)))
        else {
            log::info!("no Tauri installation of {product_name} to remove");
            return;
        };
        log::info!("removing the Tauri installation: {program}");
        // `/S`: silent. The uninstaller copies itself to a temporary place
        // and returns, so it is not waited for.
        if let Err(error) = Command::new(&program)
            .arg("/S")
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
        {
            log::error!("failed to run {program}: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_uninstaller_is_read_from_the_registry_output() {
        let output = "\r\nHKEY_CURRENT_USER\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\SBS Player\r\n    UninstallString    REG_SZ    \"C:\\Users\\a b\\AppData\\Local\\SBS Player\\uninstall.exe\"\r\n";
        assert_eq!(
            uninstaller_in(output).as_deref(),
            Some(r"C:\Users\a b\AppData\Local\SBS Player\uninstall.exe")
        );
    }

    #[test]
    fn only_an_nsis_uninstaller_is_run() {
        assert_eq!(uninstaller_in("    UninstallString    REG_SZ    \"C:\\x\\setup.exe\""), None);
        assert_eq!(uninstaller_in("nothing here"), None);
    }
}
