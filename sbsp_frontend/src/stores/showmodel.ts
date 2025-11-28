import { defineStore } from 'pinia';

import type { ShowModel } from '../types/ShowModel';
import { Cue } from '../types/Cue';
import { useUiState } from './uistate';
import { invoke } from '@tauri-apps/api/core';
import { useUiSettings } from './uiSettings';
import { v4 } from 'uuid';
import { toRaw } from 'vue';

export const useShowModel = defineStore('showmodel', {
  state: () =>
    ({
      name: '',
      cues: [],
      settings: {
        general: {
          copyAssetsDestination: '.',
        },
        audio: {
          monoOutput: false,
        },
        remote: {
          lockCursorToSelection: false,
        },
      },
    }) as ShowModel,
  actions: {
    updateAll(newModel: ShowModel) {
      this.name = newModel.name;
      this.cues = newModel.cues;
      this.settings = newModel.settings;
    },
    updateCue(newCue: Cue) {
      this.cues.splice(
        this.cues.findIndex((cue) => cue.id == newCue.id),
        1,
        newCue,
      );
    },
    addCue(cue: Cue, atIndex: number) {
      this.cues.splice(atIndex, 0, cue);
    },
    addCues(cues: Cue[], atIndex: number) {
      this.cues.splice(atIndex, 0, ...cues);
    },
    removeCue(cueId: string) {
      this.cues.splice(
        this.cues.findIndex((cue) => cue.id == cueId),
        1,
      );
    },
    moveCue(cueId: string, toIndex: number) {
      this.cues.splice(
        toIndex,
        0,
        this.cues.splice(
          this.cues.findIndex((cue) => cue.id == cueId),
          1,
        )[0],
      );
    },
    addEmptyAudioCue() {
      const uiState = useUiState();
      const uiSettings = useUiSettings();
      let insertIndex;
      if (uiState.selected) {
        insertIndex = this.cues.findIndex((cue) => cue.id == uiState.selected) + 1;
      } else {
        insertIndex = this.cues.length;
      }
      invoke<string[]>('pick_audio_assets', {})
        .then((assets) => {
          if (assets.length == 1) {
            const newCue = structuredClone(toRaw(uiSettings.settings.template.audio)) as Cue;
            newCue.id = v4();
            if (newCue.params.type == 'audio') {
              newCue.params.target = assets[0];
            }
            invoke('add_cue', { cue: newCue, atIndex: insertIndex }).catch((e) => console.error(e));
          } else if (assets.length > 1) {
            const newCues = [] as Cue[];
            for (const asset_path of assets) {
              const newCue = structuredClone(toRaw(uiSettings.settings.template.audio)) as Cue;
              newCue.id = v4();
              if (newCue.params.type == 'audio') {
                newCue.params.target = asset_path;
              }
              newCues.push(newCue);
            }
            invoke('add_cues', { cues: newCues, atIndex: insertIndex }).catch((e) => console.error(e));
          }
        })
        .catch((e) => console.error(e));
    },
    addEmptyWaitCue() {
      const uiState = useUiState();
      const uiSettings = useUiSettings();
      let insertIndex;
      if (uiState.selected) {
        insertIndex = this.cues.findIndex((cue) => cue.id == uiState.selected) + 1;
      } else {
        insertIndex = this.cues.length;
      }
      const newCue = structuredClone(toRaw(uiSettings.settings.template.wait)) as Cue;
      newCue.id = v4();
      invoke('add_cue', { cue: newCue, atIndex: insertIndex }).catch((e) => console.error(e));
    },
    addEmptyFadeCue() {
      const uiState = useUiState();
      const uiSettings = useUiSettings();
      let insertIndex;
      if (uiState.selected) {
        insertIndex = this.cues.findIndex((cue) => cue.id == uiState.selected) + 1;
      } else {
        insertIndex = this.cues.length;
      }
      const newCue = structuredClone(toRaw(uiSettings.settings.template.fade)) as Cue;
      newCue.id = v4();
      if (newCue.params.type == 'fade' && uiState.selected != null) {
        const targetCue = this.cues.find((cue) => cue.id == uiState.selected);
        if (targetCue != null && targetCue.params.type == 'audio') {
          newCue.params.target = uiState.selected;
          invoke('add_cue', { cue: newCue, atIndex: insertIndex }).catch((e) => console.error(e));
        }
      }
    },
    addEmptyPlaybackCue(type: 'start' | 'stop' | 'pause' | 'load') {
      const uiSettings = useUiSettings();
      const uiState = useUiState();
      let insertIndex;
      if (uiState.selected != null) {
        const selectedIndex = this.cues.findIndex((cue) => cue.id == uiState.selected);
        if (type == 'load' || type == 'start') {
          insertIndex = selectedIndex;
        } else {
          insertIndex = selectedIndex + 1;
        }
      } else {
        return;
      }
      let newCue;
      switch (type) {
        case 'start':
          newCue = structuredClone(toRaw(uiSettings.settings.template.start)) as Cue;
          break;
        case 'stop':
          newCue = structuredClone(toRaw(uiSettings.settings.template.stop)) as Cue;
          break;
        case 'pause':
          newCue = structuredClone(toRaw(uiSettings.settings.template.pause)) as Cue;
          break;
        case 'load':
          newCue = structuredClone(toRaw(uiSettings.settings.template.load)) as Cue;
          break;
      }
      newCue.id = v4();
      const targetCue = this.cues.find((cue) => cue.id == uiState.selected);
      if (
        targetCue != null &&
        (newCue.params.type == 'start' ||
          newCue.params.type == 'stop' ||
          newCue.params.type == 'pause' ||
          newCue.params.type == 'load')
      ) {
        newCue.params.target = uiState.selected;
        invoke('add_cue', { cue: newCue, atIndex: insertIndex }).catch((e) => console.error(e));
      }
    },
  },
});
