import { defineStore, storeToRefs } from 'pinia';
import { useShowModel } from './showmodel';
import type { AssetData } from '../types/AssetData';
import type { AssetMetadata } from '../types/AssetMetadata';
import { reactive, ref } from 'vue';
import { useApi } from '../api';

export const useAssetResult = defineStore(
  'assetResult',
  () => {
    const metadatas = ref<{ [path: string]: AssetMetadata }>({});
    const results = ref<{ [path: string]: AssetData }>({});
    const processing = reactive<Set<string>>(new Set([]));

    const add = (path: string, data: AssetData) => {
      results.value[path] = data;
      metadatas.value[path] = data.metadata;
      processing.delete(path);
    };
    const addMetadata = (path: string, data: AssetMetadata) => {
      metadatas.value[path] = data;
    };

    const get = (cueId: string | null | undefined): AssetData | null => {
      if (cueId == null) {
        return null;
      }
      const showModel = useShowModel();
      const { getCueById } = storeToRefs(showModel);
      const api = useApi();
      const targetCue = getCueById.value(cueId);
      if (targetCue != null && targetCue.params.type == 'audio') {
        const result = results.value[targetCue.params.target];
        if (result != null) {
          return result;
        } else if (!processing.has(targetCue.params.target)) {
          processing.add(targetCue.params.target);
          api.processAsset(targetCue.params.target);
          return null;
        } else {
          return null;
        }
      } else {
        return null;
      }
    };

    const getMetadata = (cueId: string | null | undefined): AssetMetadata | null => {
      if (cueId == null) {
        return null;
      }
      const showModel = useShowModel();
      const { getCueById } = storeToRefs(showModel);
      const api = useApi();
      const targetCue = getCueById.value(cueId);
      if (targetCue != null && targetCue.params.type == 'audio') {
        const result = metadatas.value[targetCue.params.target];
        if (result != null) {
          return result;
        } else if (!processing.has(targetCue.params.target)) {
          processing.add(targetCue.params.target);
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
      addMetadata,
      get,
      getMetadata,
    };
  },
  {
    persist: {
      storage: sessionStorage,
    },
  },
);
