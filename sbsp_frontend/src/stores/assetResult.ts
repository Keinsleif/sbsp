import { defineStore } from 'pinia';
import { useShowModel } from './showmodel';
import { invoke } from '@tauri-apps/api/core';
import { AssetData } from '../types/AssetData';

export const useAssetResult = defineStore('assetResult', {
  state: () => ({
    results: {} as { [path: string]: AssetData },
  }),
  actions: {
    updateAssetData() {
      const showModel = useShowModel();
      for (const cue of showModel.cues) {
        if (cue.params.type == 'audio') {
          invoke<[string, AssetData]>('process_asset', { path: cue.params.target }).catch((e) => console.error(e));
        }
      }
    },
    get(cueId: string | null | undefined): AssetData | null {
      if (cueId == null) {
        return null;
      }
      const showModel = useShowModel();
      const targetCue = showModel.cues.find((cue) => cue.id == cueId);
      if (targetCue != null && targetCue.params.type == 'audio') {
        if (targetCue.params.target in this.results) {
          return this.results[targetCue.params.target];
        } else {
          invoke<[string, AssetData]>('process_asset', { path: targetCue.params.target }).catch((e) =>
            console.error(e),
          );
          return null;
        }
      } else {
        return null;
      }
    },
  },
});
