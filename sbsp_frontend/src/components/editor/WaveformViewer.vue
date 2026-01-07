<template>
  <div
    :style="{ height: `${props.heightPx}px` }"
    class="w-100 border-md"
    @contextmenu.prevent="
      contextMenuPosition = [$event.clientX, $event.clientY];
      isContextMenuOpen = true;
    "
  >
    <v-sheet
      v-show="!isOutside"
      v-if="props.targetId != null && svgRef != null && parent != null"
      :style="tooltipStyle"
      class="pl-1 pr-1 rounded text-caption"
    >
      {{ tooltipText }}
    </v-sheet>
    <v-menu v-model="isContextMenuOpen" :target="contextMenuPosition || undefined" density="compact">
      <v-list density="compact" class="pa-0 border" @contextmenu.prevent>
        <v-list-item height="40px" density="compact">
          <v-checkbox
            v-model="uiState.scaleWaveform"
            style="font-size: 0.8em"
            :label="t('main.bottomEditor.timeLevels.scaleWaveform')"
            density="compact"
            hide-details
          ></v-checkbox>
        </v-list-item>
      </v-list>
    </v-menu>
    <svg
      ref="svg"
      preserveAspectRatio="none"
      v-show="props.targetId != null"
      xmlns="http://www.w3.org/2000/svg"
      :viewBox="`0 0 ${svgWidth} ${contentHeight}`"
      width="100%"
      :height="`${contentHeight}px`"
      @mousedown="seek($event)"
    >
      <rect :class="$style.waveform" x="0" :y="contentHeight / 2" height="1" :width="svgWidth"></rect>
      <path
        v-if="waveformPath != null"
        :d="waveformPath"
        :transform="waveformTransform"
        :class="$style.waveform"
        transform-origin="center"
      />
      <rect :x="startPos" y="0" width="2" :height="contentHeight" fill="blue"></rect>
      <rect :x="endPos" y="0" width="2" :height="contentHeight" fill="blue"></rect>
      <rect
        v-show="position != 0"
        :style="{
          transform: `translateX(${currentPlayPos}px)`,
          transition: 'transform 150ms linear',
        }"
        x="0"
        y="0"
        width="2"
        :height="contentHeight"
        fill="yellow"
      ></rect>
    </svg>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, StyleValue, useTemplateRef } from 'vue';
import { useAssetResult } from '../../stores/assetResult';
import { useShowState } from '../../stores/showstate';
import { invoke } from '@tauri-apps/api/core';
import { computedAsync, useElementSize, useMouseInElement, useParentElement } from '@vueuse/core';
import { secondsToFormat } from '../../utils';
import { useUiState } from '../../stores/uistate';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();

const showState = useShowState();
const assetResult = useAssetResult();
const uiState = useUiState();

const isContextMenuOpen = ref(false);
const contextMenuPosition = ref<[number, number] | null>(null);

const props = withDefaults(
  defineProps<{
    targetId: string | null;
    volume?: number;
    startTime?: number | null;
    endTime?: number | null;
    heightPx?: number;
  }>(),
  {
    targetId: null,
    volume: 0,
    startTime: 0,
    endTime: 1,
    heightPx: 75,
  },
);

const contentHeight = computed(() => props.heightPx - 4);

const nonNullStartTime = computed<number>(() => {
  const duration = assetResult.get(props.targetId)?.duration;
  return props.startTime != null && duration != null ? props.startTime / duration : 0;
});

const nonNullEndTime = computed<number>(() => {
  const duration = assetResult.get(props.targetId)?.duration;
  return props.endTime != null && duration != null ? props.endTime / duration : 1;
});

const startPos = computed<number>(() => nonNullStartTime.value * (svgWidth.value - 1));
const endPos = computed<number>(() => nonNullEndTime.value * (svgWidth.value - 1) - 1);

const svgRef = useTemplateRef('svg');
const { width: svgWidth } = useElementSize(svgRef);
const parent = useParentElement();
const position = computed<number>(() => {
  if (props.targetId == null) return 0;
  const activeCue = showState.activeCues[props.targetId];
  if (activeCue != null && activeCue.duration !== 0) {
    return activeCue.position / activeCue.duration;
  } else {
    return 0;
  }
});

const currentPlayPos = computed(() => {
  const range = nonNullEndTime.value - nonNullStartTime.value;
  return (nonNullStartTime.value + position.value * range) * (svgWidth.value - 1);
});

const waveformPath = computedAsync(async () => {
  if (svgWidth.value < 1) {
    return '';
  }
  if (props.targetId == null) {
    return '';
  }
  const source = assetResult.get(props.targetId)?.waveform;
  if (source == null) {
    return '';
  }

  let result = '';
  const amp = contentHeight.value * 0.375;

  const samplePerPixel = source.length / svgWidth.value;
  for (let i = 0; i < svgWidth.value; i++) {
    let start = Math.floor(i * samplePerPixel);
    let end = Math.floor((i + 1) * samplePerPixel);

    let max = source[start];
    for (let j = start; j < end; j++) {
      if (source[j] > max) max = source[j];
    }
    if (max > 0) {
      result += `M${i},${(1 - max) * amp}v${2 * amp * max}`;
    }
  }
  return result;
}, null);

const waveformTransform = computed(() => {
  if (uiState.scaleWaveform) {
    return `scale(1, ${Math.pow(10, props.volume / 20)}) translate(0, ${contentHeight.value * 0.125})`;
  } else {
    return `translate(0, ${contentHeight.value * 0.125})`;
  }
});

const { x: mouseX, y: mouseY, elementX, isOutside } = useMouseInElement(svgRef);

const seek = (event: MouseEvent) => {
  if (props.targetId == null || event.button != 0) {
    return;
  }
  const activeCue = showState.activeCues[props.targetId];
  if (activeCue == null) {
    return;
  }
  const position =
    (event.offsetX - nonNullStartTime.value * svgWidth.value) /
    (svgWidth.value * (nonNullEndTime.value - nonNullStartTime.value));
  if (position > 0 && position < 1) {
    invoke('seek_to', { cueId: props.targetId, position: position * activeCue.duration });
  }
};

const tooltipText = computed(() => {
  if (props.targetId == null) {
    return '--:--.-- / --:--.--';
  }
  const duration = assetResult.get(props.targetId)?.duration;
  if (duration == null) {
    return '--:--.-- / --:--.--';
  }
  return `${secondsToFormat((elementX.value / svgWidth.value) * duration)} / ${secondsToFormat(duration)}`;
});

const tooltipStyle = computed<StyleValue>(() => {
  if (parent.value == null) return {};
  const parentRect = parent.value.getBoundingClientRect();
  return {
    position: 'absolute',
    top: `${mouseY.value - parentRect.top - 10}px`,
    left: `${parentRect.right - mouseX.value > 150 ? mouseX.value - parentRect.left + 15 : mouseX.value - parentRect.left - 120}px`,
  };
});
</script>

<style lang="css" module>
.waveform {
  stroke: rgb(var(--v-theme-surface-variant), 0.8);
}
</style>
