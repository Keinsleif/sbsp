// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

fn main() -> iced::Result {
    sbsp_ui::logging::init("com.keinsleif.sbsp", "sbsp");
    sbsp_ui::host::run()
}
