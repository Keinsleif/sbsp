<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { computed, useTemplateRef, watchEffect } from 'vue';
import WaveformPath from '../display/WaveformPath.vue';
import { useElementSize, useMouseInElement } from '@vueuse/core';
import type { Cue } from '@/types/Cue.ts';
import { useShowState } from '@/stores/showState.ts';
import { usePosition } from '@/composables/usePosition.ts';
import { useAssetResult } from '@/stores/assetResult.ts';
import { useApi } from '@/api/index.ts';
import { useUiState } from '@/stores/uiState.ts';
import { secondsToFormat } from '@/utils.ts';

const selectedCue = defineModel<Cue | null>();

const api = useApi();
const uiState = useUiState();
const showState = useShowState();
const assetResult = useAssetResult();

const svgRef = useTemplateRef('svg');
const { width: svgWidth } = useElementSize(svgRef);

const metadata = computed(() =>
  selectedCue.value ? assetResult.getMetadata(selectedCue.value.id) : null,
);
const timeRange = computed(() => {
  const duration = metadata.value?.duration ?? 1;
  const start =
    selectedCue.value?.params.type === 'audio'
      ? (selectedCue.value.params.startTime ?? 0) / duration
      : 0;
  const end =
    selectedCue.value?.params.type === 'audio'
      ? (selectedCue.value.params.endTime ?? duration) / duration
      : 1;
  return { start, end, delta: end - start };
});
const isActive = computed(() => selectedCue.value != null && selectedCue.value.id in showState.activeCues);

const positionRef = useTemplateRef('position');
usePosition((pos) => {
  if (positionRef.value == null || selectedCue.value == null) return;
  const activeCue = showState.activeCues[selectedCue.value.id];
  let position = pos[selectedCue.value.id];
  if (activeCue == null || position == null) return;
  if (
    activeCue.duration !== 0 &&
    activeCue.status !== 'preWaiting' &&
    activeCue.status !== 'preWaitPaused'
  ) {
    position = position / activeCue.duration;
  } else {
    position = 0;
  }
  const range = timeRange.value;
  const x = (range.start + position * range.delta) * (svgWidth.value - 1);
  positionRef.value.style.transform = `translateX(${x}px)`;
});

const { x, y, elementX, isOutside } = useMouseInElement(svgRef, {
  handleOutside: false,
  touch: false,
});

const tooltipRef = useTemplateRef('tooltip');

watchEffect(() => {
  if (tooltipRef.value == null) return;
  if (!isOutside.value) {
    tooltipRef.value.style.transform = `translate(${x.value + 16}px, ${y.value}px)`;
    if (selectedCue.value == null) {
      tooltipRef.value.textContent = '--:--.-- / --:--.--';
      return;
    }
    const duration = assetResult.getMetadata(selectedCue.value.id)?.duration;
    if (duration == null) {
      tooltipRef.value.textContent = '--:--.-- / --:--.--';
    } else {
      tooltipRef.value.textContent = `${secondsToFormat((elementX.value / svgWidth.value) * duration)} / ${secondsToFormat(duration)}`;
    }
  }
});

const seek = (event: MouseEvent) => {
  if (!isActive.value || uiState.mode === 'view') return;
  if (selectedCue.value == null || event.button !== 0 || !(event.currentTarget instanceof HTMLElement)) {
    return;
  }
  const activeCue = showState.activeCues[selectedCue.value.id];
  if (activeCue == null) {
    return;
  }
  const rect = event.currentTarget.getBoundingClientRect();
  const position =
    (event.clientX + rect.left - timeRange.value.start * svgWidth.value) /
    (svgWidth.value * timeRange.value.delta);
  if (position > 0 && position < 1) {
    api.sendSeekTo(selectedCue.value.id, position * activeCue.duration);
  }
};
</script>

<template>
  <div
    class="relative w-full border border-(--p-form-field-border-color) h-16"
    ref="container"
  >
    <Teleport to="body">
      <div
        ref="tooltip"
        v-show="!isOutside"
        v-if="selectedCue != null && svgRef != null"
        class="pointer-events-none absolute top-0 left-0 rounded border border-(--p-form-field-border-color) bg-(--p-content-background) pr-1 pl-1"
      />
    </Teleport>
    <svg
      ref="svg"
      xmlns="http://www.w3.org/2000/svg"
      :viewBox="`0 0 ${svgWidth} 64`"
      preserveAspectRatio="none"
      height="100%"
      width="100%"
      @pointerdown="seek"
    >
      <waveform-path v-model="selectedCue" :height="64" :width="svgWidth" />
      <rect
        v-if="selectedCue != null && selectedCue.params.type === 'audio'"
        :x="timeRange.start * (svgWidth - 1) - 1"
        y="0"
        width="2"
        :height="64"
        fill="blue"
      />
      <rect
        v-if="selectedCue != null && selectedCue.params.type === 'audio'"
        :x="timeRange.end * (svgWidth - 1)"
        y="0"
        width="2"
        :height="64"
        fill="blue"
      />
      <rect
        v-show="isActive"
        ref="position"
        x="0"
        y="0"
        width="2"
        :height="64"
        fill="yellow"
      />
    </svg>
  </div>
</template>