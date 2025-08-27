<template>
  <div style="height: 120px" class="w-100 border-md">
    <svg
      ref="svg"
      preserveAspectRatio="none"
      v-if="targetId != null && targetId in assetResult.waveform"
      xmlns="http://www.w3.org/2000/svg"
      :viewBox="`0 0 ${compressedWaveform.length} 116`"
      width="100%"
      height="116px"
    >
      <rect
        :class="$style.waveform"
        v-for="(sample, index) in compressedWaveform"
        :key="index"
        :x="index"
        :y="(1 - sample) * 46 + 12"
        width="1"
        :height="sample * 92"
      ></rect>
      <rect
        v-if="targetId != null"
        :x="startTime * (compressedWaveform.length - 1)"
        y="0"
        width="2"
        height="116"
        fill="blue"
      ></rect>
      <rect
        v-if="targetId != null"
        :x="endTime * (compressedWaveform.length - 1)"
        y="0"
        width="2"
        height="116"
        fill="blue"
      ></rect>
      <rect
        v-if="position != null"
        :x="
          position * ((endTime - startTime) * (compressedWaveform.length - 1)) + startTime * compressedWaveform.length
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
    const window = Math.floor(assetResult.waveform[targetId.value].length / svgRef.value.clientWidth);
    for (let i = 0; i < svgRef.value.clientWidth; i++) {
      if (window == 0) {
        result.push(
          assetResult.waveform[targetId.value][
            Math.floor((assetResult.waveform[targetId.value].length * i) / svgRef.value.clientWidth)
          ],
        );
      } else {
        let start = i * window;
        let slice;
        if (assetResult.waveform[targetId.value].length < start + window) {
          slice = assetResult.waveform[targetId.value].slice(start);
        } else {
          slice = assetResult.waveform[targetId.value].slice(start, start + window);
        }
        result.push(slice.reduce((a, b) => Math.max(a, b), -Infinity));
        // result.push(slice.reduce((a, b) => Math.min(a, b), Infinity));
        // result.push(slice.reduce((acc, cur) => acc + cur, 0) / slice.length);
      }
    }
    return result;
  } else {
    return [];
  }
});

const startTime = computed(() => {
  if (targetId.value == null) {
    return 0;
  }
  if (!(targetId.value in assetResult.duration)) {
    return 0;
  }
  const targetCue = showModel.cues.find((cue) => cue.id == targetId.value);
  if (targetCue == null || targetCue.params.type != 'audio') {
    return 0;
  }
  return targetCue.params.startTime != null ? targetCue.params.startTime / assetResult.duration[targetId.value] : 0;
});

const endTime = computed(() => {
  if (targetId.value == null) {
    return 1;
  }
  if (!(targetId.value in assetResult.duration)) {
    return 1;
  }
  const targetCue = showModel.cues.find((cue) => cue.id == targetId.value);
  if (targetCue == null || targetCue.params.type != 'audio') {
    return 1;
  }
  return targetCue.params.endTime != null ? targetCue.params.endTime / assetResult.duration[targetId.value] : 1;
});
</script>

<style lang="css" module>
.waveform {
  fill: rgb(var(--v-theme-surface-variant), 0.8);
}
</style>
