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