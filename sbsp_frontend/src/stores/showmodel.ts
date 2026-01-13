import { defineStore } from 'pinia';

import type { ShowModel } from '../types/ShowModel';
import type { Cue } from '../types/Cue';
import { useUiState } from './uistate';
import { useUiSettings } from './uiSettings';
import { v4 } from 'uuid';
import { toRaw } from 'vue';
import type { CueSequence } from '../types/CueSequence';
import { useApi } from '../api';

type FlatCueEntry = {
  cue: Cue;
  level: number;
  parent: null | string;
  innerIndex: number;
  isHidden: boolean;
  isGroup: boolean;
  sequence: CueSequence;
  isSequenceOverrided: boolean;
};

const recursiveCueCheck = (
  list: Cue[],
  expandedRows: string[],
  level = 0,
  isHidden = false,
  parent: null | Cue = null,
): FlatCueEntry[] => {
  const cuelist: FlatCueEntry[] = [];

  list.forEach((cue, index) => {
    let sequence: CueSequence | null = null;
    if (parent?.params.type == 'group') {
      if (parent.params.mode.type == 'playlist') {
        if (index + 1 == list.length) {
          if (parent.params.mode.repeat) {
            sequence = { type: 'autoFollow', targetId: list[0].id };
          } else {
            sequence = { type: 'doNotContinue' };
          }
        } else {
          sequence = { type: 'autoFollow', targetId: null };
        }
      } else if (parent.params.mode.type == 'concurrency') {
        sequence = { type: 'doNotContinue' };
      }
    }
    cuelist.push({
      cue: cue,
      level: level,
      parent: parent != null ? parent.id : null,
      innerIndex: index,
      isHidden: isHidden,
      isGroup: cue.params.type == 'group',
      sequence: sequence != null ? sequence : cue.sequence,
      isSequenceOverrided: sequence != null,
    });

    if (cue.params.type == 'group') {
      const isExpanded = expandedRows.includes(cue.id);
      cuelist.push(...recursiveCueCheck(cue.params.children, expandedRows, level + 1, !isExpanded || isHidden, cue));
    }
  });

  return cuelist;
};

export const useShowModel = defineStore('showmodel', {
  state: () =>
    ({
      name: 'Untitled',
      cues: [],
      settings: {
        general: {
          copyAssetsDestination: '.',
        },
        audio: {
          monoOutput: false,
          lufsTarget: -14,
        },
        remote: {
          lockCursorToSelection: false,
        },
      },
    }) as ShowModel,
  getters: {
    getCueById() {
      return (cue_id: string): Cue | undefined => {
        const queue = [];
        let cuelist: Cue[] | undefined = this.cues;
        while (cuelist != undefined) {
          for (const cue of cuelist) {
            if (cue.id == cue_id) {
              return cue;
            }

            if (cue.params.type == 'group') {
              queue.push(cue.params.children);
            }
          }
          cuelist = queue.shift();
        }
      };
    },
    flatCueList(state) {
      const uiState = useUiState();
      return recursiveCueCheck(state.cues, uiState.expandedRows);
    },
    cueCount() {
      const queue = [];
      let cuelist: Cue[] | undefined = this.cues;
      let cueCount = 0;
      while (cuelist != undefined) {
        cueCount += cuelist.length;
        for (const cue of cuelist) {
          if (cue.params.type == 'group') {
            queue.push(cue.params.children);
          }
        }
        cuelist = queue.shift();
      }
      return cueCount;
    },
  },
  actions: {
    updateAll(newModel: ShowModel) {
      this.name = newModel.name;
      this.cues = newModel.cues;
      this.settings = newModel.settings;
    },
    addEmptyAudioCue() {
      const uiState = useUiState();
      const uiSettings = useUiSettings();
      const api = useApi();
      api
        .pickAudioAssets({ multiple: true })
        .then((assets) => {
          if (assets.length == 1) {
            const newCue = structuredClone(toRaw(uiSettings.settings.template.audio)) as Cue;
            newCue.id = v4();
            if (newCue.params.type == 'audio') {
              newCue.params.target = assets[0];
            }
            api.addCue(newCue, uiState.selected, false);
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
            api.addCues(newCues, uiState.selected, false);
          }
        })
        .catch((e) => console.error(e));
    },
    addEmptyWaitCue() {
      const uiState = useUiState();
      const uiSettings = useUiSettings();
      const api = useApi();
      const newCue = structuredClone(toRaw(uiSettings.settings.template.wait)) as Cue;
      newCue.id = v4();
      api.addCue(newCue, uiState.selected, false);
    },
    addEmptyFadeCue() {
      const uiState = useUiState();
      const uiSettings = useUiSettings();
      const api = useApi();
      const newCue = structuredClone(toRaw(uiSettings.settings.template.fade)) as Cue;
      newCue.id = v4();
      if (newCue.params.type == 'fade' && uiState.selected != null) {
        const targetCue = this.getCueById(uiState.selected);
        if (targetCue != null && (targetCue.params.type == 'audio' || targetCue.params.type == 'group')) {
          newCue.params.target = uiState.selected;
          api.addCue(newCue, uiState.selected, false);
        }
      }
    },
    addEmptyPlaybackCue(type: 'start' | 'stop' | 'pause' | 'load') {
      const uiSettings = useUiSettings();
      const uiState = useUiState();
      const api = useApi();
      if (uiState.selected == null) {
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
        api.addCue(newCue, uiState.selected, type == 'load' || type == 'start');
      }
    },
    addEmptyGroupCue() {
      const uiState = useUiState();
      const uiSettings = useUiSettings();
      const showModel = useShowModel();
      const api = useApi();
      const newCue = structuredClone(toRaw(uiSettings.settings.template.group)) as Cue;
      newCue.id = v4();
      if (newCue.params.type == 'group') {
        for (const item of showModel.flatCueList) {
          if (uiState.selectedRows.includes(item.cue.id)) {
            newCue.params.children.push(item.cue);
          }
        }
        for (const cue of newCue.params.children) {
          api.removeCue(cue.id);
        }
        api.addCue(newCue, null, false);
      }
    },
  },
});
