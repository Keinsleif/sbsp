// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { onUnmounted } from 'vue';
import { useApi } from '../api';
import { LevelMeterListener } from '../api/interface';

let listenerRegistered = false;

export const useLevelMeterListener = (listener: LevelMeterListener) => {
  if (listenerRegistered) {
    console.warn('Multiple Level Meter listener is not supported. ignoring...');
    return; // ignore more than one level meter listener
  }

  const api = useApi();
  api.listenLevelMeter(listener);

  listenerRegistered = true;

  onUnmounted(() => {
    listenerRegistered = false;
    api.unlistenLevelMeter();
  });
};
