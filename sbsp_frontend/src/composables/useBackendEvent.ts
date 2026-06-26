// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { onUnmounted } from 'vue';
import { useApi } from '../api';
import type { BackendEventListener } from '../api/interface';

export const useBackendEvent = async (listener: BackendEventListener) => {
  const api = useApi();
  let unlisten: (() => void) | null = null

  onUnmounted(() => {
    if (unlisten != null) {
      unlisten();
    }
  });

  api.onBackendEvent(listener).then((unlistenfn) => {
    unlisten = unlistenfn;
  });
};
