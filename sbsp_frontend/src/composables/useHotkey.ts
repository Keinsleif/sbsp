// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { toValue, watchEffect, type MaybeRefOrGetter } from 'vue';
import { tinykeys } from 'tinykeys';
import { isUserTyping } from '@/utils';
import { useApi } from '@/api';

export const MOD_KEY = useApi().isMacOs() ? 'Cmd' : 'Ctrl';
export const MOD_KEY_DISPLAY = useApi().isMacOs() ? '⌘' : 'Ctrl';
export type HotkeyListener = (event: KeyboardEvent) => void;

export const useHotkey = (key: MaybeRefOrGetter<string | null>, listener: HotkeyListener) => {
  watchEffect((onCleanup) => {
    const keys = toValue(key);
    if (keys == null) return;
    const unlisten = tinykeys(window, {
      [keys.trim()]: (event) => {
        if (isUserTyping(event)) return;
        listener(event);
      },
    });
    onCleanup(() => {
      unlisten();
    });
  });
};
