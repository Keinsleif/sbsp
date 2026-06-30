// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { defineStore } from 'pinia';

import type { ShowModel } from '../types/ShowModel';
import type { Cue } from '../types/Cue';
import { useUiState } from './uiState';
import { useUiSettings } from './uiSettings';
import { toRaw } from 'vue';
import type { CueChain } from '../types/CueChain';
import { useApi } from '../api';

export type FlatCueEntry = {
  cue: Cue;
  level: number;
  parent: null | string;
  innerIndex: number;
  isHidden: boolean;
  isGroup: boolean;
  chain: CueChain;
  isChainOverrided: boolean;
};

const recursiveCueCheck = (
  list: string[],
  cues: { [id: string]: Cue },
  expandedRows: string[],
  level = 0,
  isHidden = false,
  parent: null | Cue = null,
): FlatCueEntry[] => {
  const cuelist: FlatCueEntry[] = [];

  list.forEach((cueId, index) => {
    const cue = cues[cueId];
    if (cue == null) return;
    let chain: CueChain | null = null;
    if (parent?.params.type === 'group') {
      if (parent.params.mode.type === 'playlist') {
        if (index + 1 === list.length) {
          if (parent.params.mode.repeat) {
            chain = { type: 'afterComplete', targetId: list[0] || null }; // targetId will not null
          } else {
            chain = { type: 'doNotChain' };
          }
        } else {
          chain = { type: 'afterComplete', targetId: null };
        }
      } else if (parent.params.mode.type === 'concurrency') {
        chain = { type: 'doNotChain' };
      }
    }
    cuelist.push({
      cue: cue,
      level: level,
      parent: parent != null ? parent.id : null,
      innerIndex: index,
      isHidden: isHidden,
      isGroup: cue.params.type === 'group',
      chain: chain != null ? chain : cue.chain,
      isChainOverrided: chain != null,
    });

    if (cue.params.type === 'group') {
      const isExpanded = expandedRows.includes(cue.id);
      cuelist.push(
        ...recursiveCueCheck(
          cue.params.children,
          cues,
          expandedRows,
          level + 1,
          !isExpanded || isHidden,
          cue,
        ),
      );
    }
  });

  return cuelist;
};

export const useShowModel = defineStore('showModel', {
  state: () =>
    ({
      name: 'Untitled',
      cues: {},
      rootIds: [],
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
        return this.flatCueList.find((entry) => entry.cue.id === cue_id)?.cue;
      };
    },
    getSelectedCues(): Cue[] {
      const uiState = useUiState();
      return this.flatCueList
        .filter((entry) => uiState.selectedRows.has(entry.cue.id))
        .map((entry) => entry.cue);
    },
    flatCueList(state) {
      const uiState = useUiState();
      return recursiveCueCheck(state.rootIds, state.cues, uiState.expandedRows);
    },
    cueCount(state) {
      return Object.keys(state.cues).length;
    },
  },
  actions: {
    updateAll(newModel: ShowModel) {
      this.name = newModel.name;
      this.cues = newModel.cues;
      this.rootIds = newModel.rootIds;
      this.settings = newModel.settings;
    },
    addEmptyAudioCue() {
      const uiState = useUiState();
      const uiSettings = useUiSettings();
      const api = useApi();
      api
        .pickAudioAssets({ multiple: true })
        .then((assets) => {
          if (assets.length === 1) {
            const newCue = structuredClone(toRaw(uiSettings.settings.template.audio)) as Cue;
            const target = assets[0];
            if (newCue.params.type === 'audio' && target != null) {
              newCue.params.target = target;
            }
            api.addCue(newCue, uiState.selected, false).catch((e) => console.error(e));
          } else if (assets.length > 1) {
            const newCues = [] as Cue[];
            for (const asset_path of assets) {
              const newCue = structuredClone(toRaw(uiSettings.settings.template.audio)) as Cue;
              if (newCue.params.type === 'audio') {
                newCue.params.target = asset_path;
              }
              newCues.push(newCue);
            }
            api.addCues(newCues, uiState.selected, false).catch((e) => console.error(e));
          }
        })
        .catch((e) => console.error(e));
    },
    addEmptyWaitCue() {
      const uiState = useUiState();
      const uiSettings = useUiSettings();
      const api = useApi();
      const newCue = structuredClone(toRaw(uiSettings.settings.template.wait)) as Cue;
      api.addCue(newCue, uiState.selected, false).catch((e) => console.error(e));
    },
    addEmptyFadeCue() {
      const uiState = useUiState();
      const uiSettings = useUiSettings();
      const api = useApi();
      const newCue = structuredClone(toRaw(uiSettings.settings.template.fade)) as Cue;
      if (newCue.params.type === 'fade' && uiState.selected != null) {
        const targetCue = this.getCueById(uiState.selected);
        if (
          targetCue != null &&
          (targetCue.params.type === 'audio' || targetCue.params.type === 'group')
        ) {
          newCue.params.target = uiState.selected;
          api.addCue(newCue, uiState.selected, false).catch((e) => console.error(e));
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
      const targetCue = this.cues[uiState.selected];
      if (
        targetCue != null &&
        (newCue.params.type === 'start' ||
          newCue.params.type === 'stop' ||
          newCue.params.type === 'pause' ||
          newCue.params.type === 'load')
      ) {
        newCue.params.target = uiState.selected;
        api
          .addCue(newCue, uiState.selected, type === 'load' || type === 'start')
          .catch((e) => console.error(e));
      }
    },
    addEmptyGroupCue() {
      const uiState = useUiState();
      const uiSettings = useUiSettings();
      const api = useApi();
      const newCue = structuredClone(toRaw(uiSettings.settings.template.group)) as Cue;
      if (newCue.params.type === 'group') {
        api
          .addCue(newCue, uiState.selected, false)
          .then((id) => {
            if (uiState.selectedRows.size > 0) {
              api
                .moveCues(Array.from(uiState.selectedRows), {
                  type: 'inside',
                  target: id,
                  index: 0,
                })
                .catch((e) => console.error(e));
              uiState.expandedRows.push(id);
            }
          })
          .catch((e) => console.error(e));
      }
    },
  },
});
