<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { useAssetResult } from '@/stores/assetResult';
import { useUiState } from '@/stores/uiState';
import type { Cue } from '@/types/Cue';
import { useWebWorkerFn } from '@vueuse/core';
import { computed, shallowRef, toRaw, watch } from 'vue';

const selectedCue = defineModel<Cue | null>();
const props = withDefaults(defineProps<{
  volume?: number;
  width: number;
  height: number;
}>(),{
  volume: 0,
});

const assetResult = useAssetResult();
const uiState = useUiState();

const waveformPath = shallowRef('');

const buildWaveformPath = (source: number[], height: number, width: number) => {
  let result = '';
  const amp = height * 0.375;

  const samplePerPixel = source.length / width;
  for (let i = 0; i < width; i++) {
    const start = Math.floor(i * samplePerPixel);
    const end = Math.floor((i + 1) * samplePerPixel);

    let max = source[start];
    if (max == null) continue;
    for (let j = start; j < end; j++) {
      const value = source[j];
      if (value != null && value > max) max = value;
    }
    if (max > 0) {
      result += `M${i},${((1 - max) * amp).toFixed(2)}v${(2 * amp * max).toFixed()}`;
    }
  }
  return result;
};

const { workerFn, workerStatus, workerTerminate } = useWebWorkerFn(buildWaveformPath);

const updateWaveformPath = async () => {
  if (props.width < 1 || selectedCue.value == null) {
    waveformPath.value = '';
    return;
  }

  const source = assetResult.get(selectedCue.value.id)?.waveform;
  if (source == null) {
    waveformPath.value = '';
    return;
  }

  if (workerStatus.value === 'RUNNING') {
    workerTerminate();
  }

  try {
    waveformPath.value = await workerFn(toRaw(source), props.height, props.width);
  } catch (error) {
    console.error(error);
  }
};

watch(
  [() => props.width, () => props.height, () => assetResult.get(selectedCue.value?.id)?.waveform],
  (newValue, oldValue) => {
    if (newValue[2] !== oldValue[2]) {
      waveformPath.value = '';
    }
    updateWaveformPath();
  },
  { immediate: true },
);

const waveformTransform = computed(() => {
  if (uiState.scaleWaveform) {
    return `scale(1, ${Math.pow(10, props.volume / 20)}) translate(0, ${props.height * 0.125})`;
  } else {
    return `translate(0, ${props.height * 0.125})`;
  }
});

</script>

<template>
  <rect
    x="0"
    :y="props.height / 2"
    height="1"
    :width="props.width"
    fill="rgb(from var(--p-surface-500) r g b / 0.8)"
  />
  <path
    :d="waveformPath"
    :transform="waveformTransform"
    :class="$style.waveform"
    transform-origin="center"
  />
</template>

<style lang="css" module>
.waveform {
  stroke: rgb(from var(--p-surface-500) r g b / 0.8);
}
</style>