import { defineStore } from 'pinia';
import { readonly, ref } from 'vue';
import { GlobalSettings } from '../types/GlobalSettings';
import { invoke } from '@tauri-apps/api/core';

export const useUiSettings = defineStore('uiSettings', () => {
  const settings = ref<GlobalSettings>({
    general: {
      advanceCursorWhenGo: false,
      lockCursorToSelection: true,
      copyAssetsWhenAdd: false,
      seekAmount: 5.0,
    },
    hotkey: {
      playback: {
        go: 'Enter',
        load: 'L',
        pauseAndResume: 'Space',
        pauseAll: '[',
        resumeAll: ']',
        stop: 'Backspace',
        stopAll: 'Escape',
        seekForward: 'ArrowRight',
        seekBackward: 'ArrowLeft',
      },
      audioAction: {
        toggleRepeat: 'R',
      },
    },
    template: {
      audio: {
        id: '00000000-0000-0000-0000-000000000000',
        number: '',
        name: null,
        notes: '',
        preWait: 0.0,
        sequence: {
          type: 'doNotContinue',
        },
        params: {
          soundType: 'streaming',
          type: 'audio',
          target: '',
          startTime: null,
          fadeInParam: null,
          endTime: null,
          fadeOutParam: null,
          volume: 0.0,
          pan: 0.0,
          repeat: false,
        },
      },
      wait: {
        id: '00000000-0000-0000-0000-000000000000',
        number: '',
        name: null,
        notes: '',
        preWait: 0.0,
        sequence: {
          type: 'doNotContinue',
        },
        params: {
          type: 'wait',
          duration: 5.0,
        },
      },
      fade: {
        id: '00000000-0000-0000-0000-000000000000',
        number: '',
        name: null,
        notes: '',
        preWait: 0.0,
        sequence: {
          type: 'doNotContinue',
        },
        params: {
          type: 'fade',
          target: '00000000-0000-0000-0000-000000000000',
          volume: 0.0,
          fadeParam: {
            duration: 3.0,
            easing: {
              type: 'inOutPowi',
              intensity: 2,
            },
          },
        },
      },
      start: {
        id: '00000000-0000-0000-0000-000000000000',
        number: '',
        name: null,
        notes: '',
        preWait: 0.0,
        sequence: {
          type: 'doNotContinue',
        },
        params: {
          type: 'start',
          target: '00000000-0000-0000-0000-000000000000',
        },
      },
      stop: {
        id: '00000000-0000-0000-0000-000000000000',
        number: '',
        name: null,
        notes: '',
        preWait: 0.0,
        sequence: {
          type: 'doNotContinue',
        },
        params: {
          type: 'stop',
          target: '00000000-0000-0000-0000-000000000000',
        },
      },
      pause: {
        id: '00000000-0000-0000-0000-000000000000',
        number: '',
        name: null,
        notes: '',
        preWait: 0.0,
        sequence: {
          type: 'doNotContinue',
        },
        params: {
          type: 'pause',
          target: '00000000-0000-0000-0000-000000000000',
        },
      },
      load: {
        id: '00000000-0000-0000-0000-000000000000',
        number: '',
        name: null,
        notes: '',
        preWait: 0.0,
        sequence: {
          type: 'doNotContinue',
        },
        params: {
          type: 'load',
          target: '00000000-0000-0000-0000-000000000000',
        },
      },
    },
  });

  invoke<GlobalSettings>('get_settings', {})
    .then((value) => {
      settings.value = value;
    })
    .catch((e) => console.error(e));

  const update = (newSettings: GlobalSettings) => {
    settings.value = newSettings;
    invoke('set_settings', { newSettings: newSettings }).catch((e) => console.error(e));
  };

  const reload = () => {
    invoke<GlobalSettings>('reload_settings', {})
      .then((value) => {
        settings.value = value;
      })
      .catch((e) => console.error(e));
  };

  const save = () => {
    invoke('save_settings', {}).catch((e) => console.error(e));
  };

  return {
    settings: readonly(settings),
    update,
    reload,
    save,
  };
});
