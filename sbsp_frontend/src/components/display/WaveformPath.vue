<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { useAssetResult } from '@/stores/assetResult';
import { useUiState } from '@/stores/uiState';
import type { Cue } from '@/types/Cue';
import { debounce } from '@/utils';
import { onMounted, onUnmounted, ref, toRaw, watch } from 'vue';
import WaveformWorker from './waveform.worker?worker';

const selectedCue = defineModel<Cue | null>();
const props = withDefaults(
  defineProps<{
    volume?: number;
    width: number;
    height: number;
  }>(),
  {
    volume: 0,
  },
);

const assetResult = useAssetResult();
const uiState = useUiState();

const canvasRef = ref<HTMLCanvasElement | null>(null);
let worker: Worker | null = null;

let lastWaveformSource: number[] | null | undefined = undefined;

onMounted(() => {
  if (!canvasRef.value) return;

  worker = new WaveformWorker();

  const offscreen = canvasRef.value.transferControlToOffscreen();
  const dpr = window.devicePixelRatio || 1;

  worker.postMessage(
    {
      type: 'init',
      canvas: offscreen,
      width: props.width,
      height: props.height,
      dpr,
    },
    [offscreen]
  );

  syncWorkerState();
});

onUnmounted(() => {
  worker?.terminate();
});

const syncWorkerState = () => {
  if (!worker) return;

  const currentWaveform = selectedCue.value
    ? assetResult.get(selectedCue.value.id)?.waveform
    : null;

  if (currentWaveform !== lastWaveformSource) {
    lastWaveformSource = currentWaveform;
    worker.postMessage({
      type: 'updateData',
      waveform: currentWaveform ? toRaw(currentWaveform) : null,
    });
  }

  worker.postMessage({
    type: 'render',
    volume: props.volume,
  });
};

const debouncedResize = debounce(() => {
  if (!worker) return;
  worker.postMessage({
    type: 'render',
    volume: props.volume,
    scaleWaveform: uiState.scaleWaveform,
    width: props.width,
    height: props.height,
  });
}, 100);

// --- Watcher ---

watch(
  [
    () => assetResult.get(selectedCue.value?.id)?.waveform,
    () => props.volume,
    () => uiState.scaleWaveform,
  ],
  () => {
    syncWorkerState();
  },
  { immediate: true }
);

watch([() => props.width, () => props.height], () => {
  debouncedResize();
});
</script>

<template>
  <foreignObject x="0" y="0" :width="props.width" :height="props.height">
    <canvas
      ref="canvasRef"
      :style="{
        width: `${props.width}px`,
        height: `${props.height}px`,
        display: 'block',
      }"
    />
  </foreignObject>
</template>