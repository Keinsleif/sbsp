import { defineStore } from 'pinia';
import { readonly, ref, toRaw } from 'vue';
import { GlobalSettings } from '../types/GlobalSettings';
import { useApi } from '../api';

export const useUiSettings = defineStore('uiSettings', () => {
  const settings = ref<GlobalSettings>({
    general: {
      advanceCursorWhenGo: false,
      lockCursorToSelection: true,
      copyAssetsWhenAdd: false,
      seekAmount: 5.0,
    },
    appearance: {
      language: null,
      darkMode: 'dark',
      hideControls: false,
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
      group: {
        id: '00000000-0000-0000-0000-000000000000',
        number: '',
        name: null,
        notes: '',
        preWait: 0.0,
        sequence: {
          type: 'doNotContinue',
        },
        params: {
          type: 'group',
          mode: {
            type: 'playlist',
            repeat: true,
          },
          children: [],
        },
      },
    },
    nameFormat: {
      audio: '{filename}',
      wait: 'Wait {duration}',
      fade: 'Fade {targetName}',
      start: 'Start {targetName}',
      stop: 'Stop {targetName}',
      pause: 'Pause {targetName}',
      load: 'Load {targetName}',
      group: 'Group',
    },
  });

  const api = useApi();
  api
    .getSettings()
    .then((value) => {
      settings.value = value;
    })
    .catch((e) => console.error(e));

  const update = (newSettings: GlobalSettings) => {
    settings.value = newSettings;
    api.setSettings(newSettings);
  };

  const reload = () => {
    api
      .reloadSettings()
      .then((value) => {
        settings.value = value;
      })
      .catch((e) => console.error(e));
  };

  const save = () => {
    api.saveSettings();
  };

  const clone = () => {
    return structuredClone(toRaw(settings.value));
  };

  const import_from_file = () => {
    api
      .importSettingsFromFile()
      .then((settings) => {
        update(settings);
      })
      .catch((e) => console.error(e));
  };

  const export_to_file = () => {
    api.exportSettingsToFile();
  };

  return {
    settings: readonly(settings),
    update,
    reload,
    save,
    clone,
    import_from_file,
    export_to_file,
  };
});
