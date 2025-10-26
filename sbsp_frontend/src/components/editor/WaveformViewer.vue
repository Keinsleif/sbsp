<template>
  <div style="height: 120px" class="w-100 border-md">
    <v-sheet
      v-show="!isOutside"
      v-if="props.targetId != null && svgRef != null && parent != null"
      :style="{
        position: 'absolute',
        top: `${mouseY - parent.getBoundingClientRect().top - 10}px`,
        left: `${parent.getBoundingClientRect().right - mouseX > 150 ? mouseX - parent.getBoundingClientRect().left + 15 : mouseX - parent.getBoundingClientRect().left - 120}px`,
      }"
      class="pl-1 pr-1 rounded text-caption"
    >
      {{ buildTooltipText() }}
    </v-sheet>
    <svg
      ref="svg"
      preserveAspectRatio="none"
      v-if="props.targetId != null && props.targetId in assetResult.results"
      xmlns="http://www.w3.org/2000/svg"
      :viewBox="`0 0 ${compressedWaveform.length} 116`"
      width="100%"
      height="116px"
      @mousedown="seek($event)"
    >
      <rect :class="$style.waveform" x="0" y="58" height="1" :width="compressedWaveform.length"></rect>
      <g :transform="`scale(1, ${Math.pow(10, props.volume / 20)}) translate(0, 12)`" transform-origin="center">
        <template v-for="(sample, index) in compressedWaveform" :key="index">
          <rect
            v-if="sample != 0"
            :class="$style.waveform"
            :x="index"
            :y="(1 - sample) * 46"
            width="1"
            :height="sample * 92"
          ></rect>
        </template>
      </g>
      <rect :x="nonNullStartTime * (compressedWaveform.length - 1)" y="0" width="2" height="116" fill="blue"></rect>
      <rect :x="nonNullEndTime * (compressedWaveform.length - 1) - 1" y="0" width="2" height="116" fill="blue"></rect>
      <rect
        v-show="position != 0"
        :style="{
          transform: `translateX(${
            position * ((nonNullEndTime - nonNullStartTime) * (compressedWaveform.length - 1)) +
            nonNullStartTime * compressedWaveform.length
          }px)`,
          transition: 'transform 100ms linear',
        }"
        x="0"
        y="0"
        width="2"
        height="116"
        fill="yellow"
      ></rect>
    </svg>
  </div>
</template>

<script setup lang="ts">
import { computed, useTemplateRef } from 'vue';
import { useAssetResult } from '../../stores/assetResult';
import { useShowState } from '../../stores/showstate';
import type { PlaybackStatus } from '../../types/PlaybackStatus';
import { invoke } from '@tauri-apps/api/core';
import { useMouseInElement, useParentElement } from '@vueuse/core';
import { secondsToFormat } from '../../utils';

const showState = useShowState();
const assetResult = useAssetResult();

const props = withDefaults(
  defineProps<{
    targetId: string | null;
    volume?: number;
    startTime?: number | null;
    endTime?: number | null;
  }>(),
  {
    targetId: null,
    volume: 0,
    startTime: 0,
    endTime: 1,
  },
);

const nonNullStartTime = computed<number>(() => {
  if (props.targetId == null) {
    return 0;
  }
  const duration = assetResult.results[props.targetId].duration;
  return props.startTime != null && duration != null ? props.startTime / duration : 0;
});

const nonNullEndTime = computed<number>(() => {
  if (props.targetId == null) {
    return 1;
  }
  const duration = assetResult.results[props.targetId].duration;
  return props.endTime != null && duration != null ? props.endTime / duration : 1;
});

const svgRef = useTemplateRef('svg');
const parent = useParentElement();
const position = computed<number>(() => {
  if (props.targetId != null && props.targetId in showState.activeCues) {
    const activeCue = showState.activeCues[props.targetId];
    if (
      activeCue != null &&
      (['Playing', 'Paused', 'Loaded'] as PlaybackStatus[]).includes(activeCue.status) &&
      activeCue.duration !== 0
    ) {
      return activeCue.position / activeCue.duration;
    } else {
      return 0;
    }
  } else {
    return 0;
  }
});
const compressedWaveform = computed<number[]>(() => {
  if (svgRef.value == null || svgRef.value.clientWidth < 1) {
    return [0];
  }
  if (props.targetId == null || !(props.targetId in assetResult.results)) {
    return Array(svgRef.value.clientWidth).fill(0);
  }
  let result = [] as number[];
  let source = assetResult.results[props.targetId].waveform;
  if (source == null) {
    return Array(svgRef.value.clientWidth).fill(0);
  }
  const window = source.length / (svgRef.value.clientWidth - 1);
  let loop_count = 0;
  for (let i = 0; i < source.length || loop_count > 10000; i += window) {
    loop_count++;
    let start = Math.floor(i);
    let end = Math.floor(i + window);
    if (start == end) {
      result.push(source[start]);
      continue;
    }
    let slice;
    if (source.length < end) {
      slice = source.slice(start);
    } else {
      slice = source.slice(start, end);
    }
    result.push(slice.reduce((a, b) => Math.max(a, b), -Infinity));
    // result.push(slice.reduce((a, b) => Math.min(a, b), Infinity));
    // result.push(slice.reduce((acc, cur) => acc + cur, 0) / slice.length);
  }
  return result;
});
const { x: mouseX, y: mouseY, elementX, isOutside } = useMouseInElement(svgRef);

const seek = (event: MouseEvent) => {
  if (
    svgRef.value == null ||
    svgRef.value.clientWidth < 1 ||
    props.targetId == null ||
    !(props.targetId in showState.activeCues)
  ) {
    return;
  }
  const activeCue = showState.activeCues[props.targetId];
  if (activeCue == null || !(['Playing', 'Paused', 'Loaded'] as PlaybackStatus[]).includes(activeCue.status)) {
    return;
  }
  const position =
    (event.offsetX - nonNullStartTime.value * svgRef.value.clientWidth) /
    (svgRef.value.clientWidth * (nonNullEndTime.value - nonNullStartTime.value));
  if (position > 0 && position < 1) {
    invoke('seek_to', { cueId: props.targetId, position: position * activeCue.duration });
  }
};

const buildTooltipText = () => {
  if (props.targetId == null || svgRef.value == null) {
    return '--:--.-- / --:--.--';
  }
  const duration = assetResult.results[props.targetId].duration;
  if (duration == null) {
    return '--:--.-- / --:--.--';
  }
  return `${secondsToFormat((elementX.value / svgRef.value.clientWidth) * duration)} / ${secondsToFormat(duration)}`;
};
</script>

<style lang="css" module>
.waveform {
  fill: rgb(var(--v-theme-surface-variant), 0.8);
}
</style>
