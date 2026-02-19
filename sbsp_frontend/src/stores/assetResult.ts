import { defineStore } from 'pinia';
import { useShowModel } from './showmodel';
import type { AssetData } from '../types/AssetData';
import { ref } from 'vue';
import { useApi } from '../api';

export const useAssetResult = defineStore(
  'assetResult',
  () => {
    const results = ref<{ [path: string]: AssetData }>({});
    const processing = ref<string[]>([]);

    const add = (path: string, data: AssetData) => {
      results.value[path] = data;
      processing.value.splice(processing.value.indexOf(path), 1);
    };
    const get = (cueId: string | null | undefined): AssetData | null => {
      if (cueId == null) {
        return null;
      }
      const showModel = useShowModel();
      const api = useApi();
      const targetCue = showModel.getCueById(cueId);
      if (targetCue != null && targetCue.params.type == 'audio') {
        const result = results.value[targetCue.params.target];
        if (result != null) {
          return result;
        } else if (!processing.value.includes(targetCue.params.target)) {
          processing.value.push(targetCue.params.target);
          api.processAsset(targetCue.params.target);
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
