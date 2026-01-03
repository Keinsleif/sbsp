import { defineConfig } from 'vitepress'

// https://vitepress.dev/reference/site-config
export default defineConfig({
  title: "SBS Player",
  description: "イベント向け音源再生ソフトウェア",

  markdown: {
    breaks: true
  },

  themeConfig: {
    // https://vitepress.dev/reference/default-theme-config
    nav: [
      { text: 'ホーム', link: '/' },
      { text: 'インストール', link: '/installation'},
      { text: '使用方法', link: '/usage/' },
    ],

    sidebar: [
      {
        text: 'UI',
        items: [
          { text: '外観', link: '/usage/ui/overview' }
        ]
      },
      {
        text: 'Cues',
        items: [
        ]
      }
    ],

    socialLinks: [
      { icon: 'github', link: 'https://github.com/Keinsleif/sbsp' }
    ]
  }
})
