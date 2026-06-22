// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { onUnmounted, toValue, watch, type MaybeRef } from "vue";
import { tinykeys } from "tinykeys";

export type HotkeyListener = (event: KeyboardEvent) => void;

const normalize = (key: string) => key.trim().toLowerCase().replace('ctrl', '$mod').replace('cmd', '$mod');

export const useHotkey = (key: MaybeRef<string | null>, listener: HotkeyListener) => {
 
  let unlisten: (() => void) | null = null;

  watch(() => toValue(key), (newKey) => {
    unlisten?.();
    if (newKey != null) {
      unlisten = tinykeys(window, {
        [normalize(newKey)]: listener,
      })
    }
  }, { immediate: true });
  
  onUnmounted(() => unlisten?.());
}
