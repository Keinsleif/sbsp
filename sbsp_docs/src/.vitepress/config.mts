import { defineConfig } from 'vitepress'

// https://vitepress.dev/reference/site-config
export default defineConfig({
  title: "SBS Player",
  description: "イベント向け音源再生ソフトウェア",
  head: [
    ['link', { rel: 'icon', href: '/sbsp/docs/favicon.ico', sizes: '48x48' }],
    ['link', { rel: 'icon', href: '/sbsp/docs/favicon.svg', sizes: 'any', type: 'image/svg+xml' }],
    ['link', { rel: 'apple-touch-icon', href: '/sbsp/docs/apple-touch-icon.png'}]
  ],
  base: '/sbsp/docs/',

  markdown: {
    breaks: true
  },

  themeConfig: {
    // https://vitepress.dev/reference/default-theme-config
    nav: [
      { text: 'ホーム', link: '/' },
      { text: 'インストール', link: '/installation'},
      { text: '使用方法', link: '/usage/ui/main' },
    ],

    sidebar: [
      {
        text: 'インターフェース',
        items: [
          { text: 'メイン画面', link: '/usage/ui/main' },
          {
            text: 'エディタ',
            link: '/usage/ui/editor/basic_editor',
            collapsed: true,
            items: [
              { text: '基本', link: '/usage/ui/editor/basic_editor' },
              { text: '音声キュー', link: '/usage/ui/editor/audio' },
              { text: 'フェードキュー', link: '/usage/ui/editor/fade' },
              { text: '再生制御キュー', link: '/usage/ui/editor/playback' },
              { text: 'グループキュー', link: '/usage/ui/editor/group' },
            ],
          },
          { text: '設定画面', link: '/usage/ui/settings' },
        ],
      },
    ],

    socialLinks: [
      { icon: 'github', link: 'https://github.com/Keinsleif/sbsp' }
    ]
  }
})
