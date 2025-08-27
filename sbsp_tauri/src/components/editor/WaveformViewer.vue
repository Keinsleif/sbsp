<template>
  <div style="height: 120px" class="w-100 border-md">
    <svg
      ref="svg"
      preserveAspectRatio="none"
      v-if="
        targetId != null && targetCue != null && targetCue.params.type == 'audio' && targetId in assetResult.waveform
      "
      xmlns="http://www.w3.org/2000/svg"
      :viewBox="`0 0 ${compressedWaveform.length} 116`"
      width="100%"
      height="116px"
    >
      <g :transform="`scale(1, ${Math.pow(10, props.volume / 20)}) translate(0, 12)`" transform-origin="center">
        <rect
          :class="$style.waveform"
          v-for="(sample, index) in compressedWaveform"
          :key="index"
          :x="index"
          :y="(1 - sample) * 46"
          width="1"
          :height="sample * 92"
        ></rect>
      </g>
      <rect :x="nonNullStartTime * (compressedWaveform.length - 1)" y="0" width="2" height="116" fill="blue"></rect>
      <rect :x="nonNullEndTime * (compressedWaveform.length - 1) - 1" y="0" width="2" height="116" fill="blue"></rect>
      <rect
        v-if="position != null"
        :x="
          position * ((nonNullEndTime - nonNullStartTime) * (compressedWaveform.length - 1)) +
          nonNullStartTime * compressedWaveform.length
        "
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
import { useShowModel } from '../../stores/showmodel';

const showModel = useShowModel();
const showState = useShowState();
const assetResult = useAssetResult();

const targetId = defineModel<string | null>({ required: true });
const props = withDefaults(
  defineProps<{
    volume?: number;
    startTime?: number | null;
    endTime?: number | null;
    duration?: number;
  }>(),
  {
    volume: 0,
    startTime: 0,
    endTime: 1,
    duration: 0,
  },
);

const nonNullStartTime = computed(() => {
  return props.startTime != null ? props.startTime / props.duration : 0;
});

const nonNullEndTime = computed(() => {
  return props.endTime != null ? props.endTime / props.duration : 1;
});

const targetCue = computed(() => {
  return targetId.value != null ? showModel.cues.find((cue) => cue.id === targetId.value) : null;
});

const svgRef = useTemplateRef('svg');
const position = computed(() => {
  if (
    targetId.value != null &&
    targetId.value in showState.activeCues &&
    (['Playing', 'Paused'] as PlaybackStatus[]).includes(showState.activeCues[targetId.value]!.status)
  ) {
    return showState.activeCues[targetId.value]!.position / showState.activeCues[targetId.value]!.duration;
  } else {
    return null;
  }
});
const compressedWaveform = computed(() => {
  if (svgRef.value != null && targetId.value != null && targetId.value in assetResult.waveform) {
    let result = [];
    let source = assetResult.waveform[targetId.value];
    const window = source.length / (svgRef.value.clientWidth - 1);
    for (let i = 0; i < source.length; i += window) {
      let start = Math.floor(i);
      let end = Math.floor(i + window);
      let slice;
      if (start == end) {
        result.push(source[start]);
        continue;
      } else if (source.length < end) {
        slice = source.slice(start);
      } else {
        slice = source.slice(start, end);
      }
      result.push(slice.reduce((a, b) => Math.max(a, b), -Infinity));
      // result.push(slice.reduce((a, b) => Math.min(a, b), Infinity));
      // result.push(slice.reduce((acc, cur) => acc + cur, 0) / slice.length);
    }
    return result;
  } else {
    return [];
  }
});
</script>

<style lang="css" module>
.waveform {
  fill: rgb(var(--v-theme-surface-variant), 0.8);
}
/* .draggable {
  cursor: col-resize;
} */
</style>
