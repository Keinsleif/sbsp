import { defineStore } from 'pinia';

import type { ShowModel } from '../types/ShowModel';
import { Cue } from '../types/Cue';

export const useShowModel = defineStore('showmodel', {
  state: () =>
    ({
      name: '',
      cues: [],
      settings: {
        general: { lockCursorToSelection: true, advanceCursorWhenGo: true },
        hotkey: {
          go: 'Enter',
          load: 'L',
          pauseAndResume: 'Space',
          pauseAll: '[',
          resumeAll: ']',
          stop: 'Backspace',
          stopAll: 'Escape',
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
        },
        audio: {
          monoOutput: false,
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
  },
});
