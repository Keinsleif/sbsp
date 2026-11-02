// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import type { Theme } from 'vitepress'
import DefaultTheme from 'vitepress/theme'
import { VPButton } from 'vitepress/theme'

export default {
  extends: DefaultTheme,
  enhanceApp({ app }) {
    // register your custom global components
    app.component('VPButton', VPButton)
  }
} satisfies Theme