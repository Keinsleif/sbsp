// https://vitepress.dev/reference/site-config
const enConfig = {
  label: 'English',
  lang: 'en',
  description: 'Audio playback software for events usage',
  themeConfig: {
    // https://vitepress.dev/reference/default-theme-config
    nav: [
      { text: 'Home', link: '/' },
      { text: 'Installation', link: '/installation' },
      { text: 'Usage', link: '/usage/ui/main' },
    ],

    sidebar: [
      {
        text: 'Interface',
        items: [
          { text: 'Main', link: '/usage/ui/main' },
          {
            text: 'Editor',
            link: '/usage/ui/editor/basic_editor',
            collapsed: true,
            items: [
              { text: 'Basic', link: '/usage/ui/editor/basic_editor' },
              { text: 'Audio Cue', link: '/usage/ui/editor/audio' },
              { text: 'Fade Cue', link: '/usage/ui/editor/fade' },
              { text: 'Playback Cues', link: '/usage/ui/editor/playback' },
              { text: 'Group Cue', link: '/usage/ui/editor/group' },
            ],
          },
          { text: 'Settings', link: '/usage/ui/settings' },
        ],
      },
      {
        text: 'Tips',
        items: [{ text: 'Precise clock', link: '/tips/clock' }],
      },
    ],

    socialLinks: [{ icon: 'github', link: 'https://github.com/Keinsleif/sbsp' }],
  },
};

export default enConfig;
