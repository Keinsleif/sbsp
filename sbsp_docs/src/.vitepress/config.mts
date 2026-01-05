import { defineConfig } from 'vitepress'

// https://vitepress.dev/reference/site-config
export default defineConfig({
  title: "SBS Player",
  description: "イベント向け音源再生ソフトウェア",
  head: [
    ['link', { rel: 'icon', href: '/favicon.ico', sizes: '48x48' }],
    ['link', { rel: 'icon', href: '/favicon.svg', sizes: 'any', type: 'image/svg+xml' }],
    ['link', { rel: 'apple-touch-icon', href: '/apple-touch-icon.png'}]
  ],

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
          { text: 'メイン画面', link: '/usage/ui/main' }
        ]
      },
      {
        text: 'Cues',
        items: [
          { text: '音声キュー', link: '/usage/cues/audio' }
        ]
      }
    ],

    socialLinks: [
      { icon: 'github', link: 'https://github.com/Keinsleif/sbsp' }
    ]
  }
})
