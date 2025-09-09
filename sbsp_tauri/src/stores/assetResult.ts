import { defineStore } from 'pinia';
import { useShowModel } from './showmodel';
import { invoke } from '@tauri-apps/api/core';
import { AssetData } from '../types/AssetData';

export const useAssetResult = defineStore('assetResult', {
  state: () => ({
    results: {} as { [cue_id: string]: AssetData },
  }),
  actions: {
    updateAssetData() {
      const showModel = useShowModel();
      for (const cue of showModel.cues) {
        if (cue.params.type == 'audio') {
          invoke<[string, AssetData]>('process_asset', { cueId: cue.id }).then((value) => {
            this.results[cue.id] = value[1];
          });
        }
      }
    },
  },
});
