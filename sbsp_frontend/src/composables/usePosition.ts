// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { onUnmounted, unref, watch, type MaybeRef } from 'vue';
import { useShowState } from '../stores/showState';

type PositionCallback = (positions: { [id: string]: number }) => void;

const callbacks = new Set<PositionCallback>();
let rafId: number | null = null;
let lastFlush = 0;
let tickerUsers = 0;

let showState: ReturnType<typeof useShowState> | null = null;

const loop = (timestamp: DOMHighResTimeStamp) => {
  if (showState == null) return;
  let positions;

  // 100ms for reactive update
  if (timestamp - lastFlush > 100) {
    positions = showState.calculatePosition(true);
    lastFlush = timestamp;
  } else {
    positions = showState.calculatePosition(false);
  }

  callbacks.forEach((fn) => fn(positions));

  rafId = requestAnimationFrame(loop);
};

export const usePosition = (domTickFn: PositionCallback, enabled: MaybeRef<boolean> = true, onDeactivate?: () => void) => {
  if (showState == null) {
    showState = useShowState();
  }
  watch(
    () => unref(enabled),
    (isActive, _, onCleanup) => {
      if (isActive && domTickFn) {
        callbacks.add(domTickFn);

        onCleanup(() => {
          callbacks.delete(domTickFn);
          onDeactivate?.();
        });
      }
    },
    { immediate: true },
  );

  if (rafId == null) {
    rafId = requestAnimationFrame(loop);
  }

  onUnmounted(() => {
    callbacks.delete(domTickFn);
    if (callbacks.size === 0 && tickerUsers === 0 && rafId != null) {
      cancelAnimationFrame(rafId);
      rafId = null;
    }
  });
};

export const usePositionTicker = () => {
  if (showState == null) {
    showState = useShowState();
  }
  tickerUsers++;

  if (rafId == null) {
    rafId = requestAnimationFrame(loop);
  }

  onUnmounted(() => {
    tickerUsers--;
    if (callbacks.size === 0 && tickerUsers === 0 && rafId != null) {
      cancelAnimationFrame(rafId);
      rafId = null;
    }
  });
};
