// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { defineStore, storeToRefs } from 'pinia';
import { useShowModel } from './showModel';
import type { AssetData } from '../types/AssetData';
import type { AssetMetadata } from '../types/AssetMetadata';
import { reactive, ref } from 'vue';
import { useApi } from '../api';

export const useAssetResult = defineStore('assetResult', () => {
  const metadatas = ref<{ [path: string]: AssetMetadata }>({});
  const results = ref<{ [path: string]: AssetData }>({});
  const processing = reactive<Set<string>>(new Set([]));
  const failed = reactive<Set<string>>(new Set([]));

  const add = (path: string, data: AssetData) => {
    results.value[path] = data;
    metadatas.value[path] = data.metadata;
    processing.delete(path);
  };
  const addError = (path: string) => {
    failed.add(path);
    processing.delete(path);
  };
  const addMetadata = (path: string, data: AssetMetadata) => {
    metadatas.value[path] = data;
  };

  const resetError = (path: string) => {
    failed.delete(path);
  };

  const requestProcess = (path: string) => {
    if (!processing.has(path) && !failed.has(path)) {
      const api = useApi();
      processing.add(path);
      api.processAsset(path).catch((e) => {
        console.error(e);
        addError(path);
      });
    }
  };

  const get = (cueId: string | null | undefined): AssetData | null => {
    if (cueId == null) {
      return null;
    }
    const showModel = useShowModel();
    const { getCueById } = storeToRefs(showModel);
    const targetCue = getCueById.value(cueId);
    if (targetCue != null && targetCue.params.type === 'audio') {
      const result = results.value[targetCue.params.target];
      if (result != null) {
        return result;
      } else {
        requestProcess(targetCue.params.target);
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
    const targetCue = getCueById.value(cueId);
    if (targetCue != null && targetCue.params.type === 'audio') {
      const result = metadatas.value[targetCue.params.target];
      if (result != null) {
        return result;
      } else {
        requestProcess(targetCue.params.target);
        return null;
      }
    } else {
      return null;
    }
  };

  return {
    results,
    processing,
    failed,
    add,
    addError,
    addMetadata,
    resetError,
    requestProcess,
    get,
    getMetadata,
  };
});
