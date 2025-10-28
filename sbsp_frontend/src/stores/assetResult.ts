import { defineStore } from 'pinia';
import { useShowModel } from './showmodel';
import { invoke } from '@tauri-apps/api/core';
import type { AssetData } from '../types/AssetData';
import { ref } from 'vue';

export const useAssetResult = defineStore(
  'assetResult',
  () => {
    const results = ref<{ [path: string]: AssetData }>({});
    const processing = ref<string[]>([]);

    const updateAssetData = () => {
      const showModel = useShowModel();
      for (const cue of showModel.cues) {
        if (cue.params.type == 'audio') {
          invoke<[string, AssetData]>('process_asset', { path: cue.params.target }).catch((e) => console.error(e));
        }
      }
    };
    const add = (path: string, data: AssetData) => {
      results.value[path] = data;
      processing.value.splice(processing.value.indexOf(path), 1);
    };
    const get = (cueId: string | null | undefined): AssetData | null => {
      if (cueId == null) {
        return null;
      }
      const showModel = useShowModel();
      const targetCue = showModel.cues.find((cue) => cue.id == cueId);
      if (targetCue != null && targetCue.params.type == 'audio') {
        if (targetCue.params.target in results.value) {
          return results.value[targetCue.params.target];
        } else if (!processing.value.includes(targetCue.params.target)) {
          processing.value.push(targetCue.params.target);
          invoke('process_asset', { path: targetCue.params.target }).catch((e) => console.error(e));
          return null;
        } else {
          return null;
        }
      } else {
        return null;
      }
    };

    return {
      results,
      processing,
      updateAssetData,
      add,
      get,
    };
  },
  {
    persist: {
      storage: sessionStorage,
    },
  },
);
