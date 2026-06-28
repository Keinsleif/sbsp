<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { computed, ref, shallowRef, toRaw, useTemplateRef, watch, watchEffect } from 'vue';
import { useAssetResult } from '../../stores/assetResult';
import { useShowState } from '../../stores/showState';
import { useElementSize, useEventListener, useMouseInElement, useWebWorkerFn } from '@vueuse/core';
import { secondsToFormat } from '../../utils';
import type { Cue } from '../../types/Cue';
import {
  mdiCheckboxBlankOutline,
  mdiCheckboxMarked,
  mdiEye,
  mdiEyeOff,
  mdiMinus,
  mdiPlus,
  mdiSkipNext,
  mdiSkipPrevious,
  mdiTrashCan,
} from '@mdi/js';
import { useApi } from '../../api';
import { useI18n } from 'vue-i18n';
import TimeInput from '../input/TimeInput.vue';
import { useUiState } from '../../stores/uiState';
import { usePosition } from '../../composables/usePosition.ts';
import ButtonWrapper from '../wrapper/ButtonWrapper.vue';
import ContextMenu from 'primevue/contextmenu';
import PathIcon from '../display/PathIcon.vue';

const { t } = useI18n();
const api = useApi();
const showState = useShowState();
const assetResult = useAssetResult();
const uiState = useUiState();

const selectedCue = defineModel<Cue | null>();
const emit = defineEmits(['update']);
const envelopeParent = useTemplateRef('parent');
const props = withDefaults(
  defineProps<{
    heightPx?: number;
    volume?: number;
    disabled?: boolean;
  }>(),
  {
    heightPx: 75,
    volume: 0,
    disabled: false,
  },
);

const MIN_GAP = 0.005;

type Segment = {
  start: number;
  end: number;
  volume: number;
};

const normSegments = (seg: Segment[]): Segment[] => {
  const result = [...seg].sort((a, b) => a.start - b.start);
  if (result.length > 0) {
    result[0]!.start = 0;
    result[result.length - 1]!.end = 1;
  }
  return result;
};

const buildTimeRange = () => {
  const duration = metadata.value?.duration || 1;
  const start =
    selectedCue.value?.params.type === 'audio'
      ? (selectedCue.value.params.startTime || 0) / duration
      : 0;
  const end =
    selectedCue.value?.params.type === 'audio'
      ? (selectedCue.value.params.endTime || duration) / duration
      : 1;
  return { start, end, delta: end - start };
};

const dragging = ref<{
  index: number;
  type: 'volume' | 'start' | 'end' | 'hstart' | 'hend';
  dragged: boolean;
} | null>(null);
const selectedIdx = ref<number | null>(null);
const segments = ref<Segment[]>(
  selectedCue.value != null && selectedCue.value.params.type === 'audio'
    ? normSegments(selectedCue.value.params.envelope)
    : [],
);

watch(selectedCue, (newCue, oldCue) => {
  if (
    newCue?.id !== oldCue?.id ||
    (selectedIdx.value != null &&
      newCue?.params.type === 'audio' &&
      newCue.params.envelope.length <= selectedIdx.value)
  ) {
    selectedIdx.value = null;
  }
  dragging.value = null;
  segments.value =
    selectedCue.value != null && selectedCue.value.params.type === 'audio'
      ? normSegments(selectedCue.value.params.envelope)
      : [];
  timeRange.value = buildTimeRange();
});

const contentHeight = computed(() => props.heightPx - 4);
const metadata = computed(() =>
  selectedCue.value ? assetResult.getMetadata(selectedCue.value.id) : null,
);

const timeRange = ref<{
  start: number;
  end: number;
  delta: number;
}>(buildTimeRange());

const startPos = computed<number>(() => timeRange.value.start * (svgWidth.value - 1));
const endPos = computed<number>(() => timeRange.value.end * (svgWidth.value - 1) - 1);

const svgRef = useTemplateRef('svg');
const { width: svgWidth } = useElementSize(svgRef);
const parent = useTemplateRef('container');

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
  if (svgWidth.value < 1 || selectedCue.value == null) {
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
    waveformPath.value = await workerFn(toRaw(source), contentHeight.value, svgWidth.value);
  } catch (error) {
    console.error(error);
  }
};

watch(
  [svgWidth, contentHeight, () => assetResult.get(selectedCue.value?.id)?.waveform],
  (newValue, oldValue) => {
    if (newValue[2] !== oldValue[2]) {
      waveformPath.value = '';
    }
    updateWaveformPath();
  },
);

const waveformTransform = computed(() => {
  if (uiState.scaleWaveform) {
    return `scale(1, ${Math.pow(10, props.volume / 20)}) translate(0, ${contentHeight.value * 0.125})`;
  } else {
    return `translate(0, ${contentHeight.value * 0.125})`;
  }
});

const { elementX, elementY, isOutside } = useMouseInElement(svgRef, {
  handleOutside: false,
  touch: false,
});

const tooltipRef = useTemplateRef('tooltip');

watchEffect(() => {
  if (tooltipRef.value == null) return;
  if (!isOutside.value) {
    tooltipRef.value.style.transform = `translate(${svgWidth.value - elementX.value > 180 ? elementX.value + 15 : elementX.value - 150}px, ${elementY.value}px)`;
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

const saveEditorValue = () => {
  if (props.disabled) return;
  if (selectedCue.value?.params.type !== 'audio') return;
  selectedCue.value.params.envelope = segments.value;

  const duration = metadata.value?.duration || 1;
  selectedCue.value.params.startTime =
    timeRange.value.start === 0 ? null : timeRange.value.start * duration;
  selectedCue.value.params.endTime =
    timeRange.value.end === 1 ? null : timeRange.value.end * duration;
  emit('update');
};

const linePath = computed<{
  dot: string;
  fill: string;
}>(() => {
  if (segments.value.length === 0)
    return {
      dot: '',
      fill: '',
    };

  return {
    dot: segments.value
      .map((value, i) => {
        const y = decibelsToY(value.volume);
        if (i === 0) {
          return `M${value.end * svgWidth.value},${y}`;
        } else if (i === segments.value.length - 1) {
          return `L${value.start * svgWidth.value},${y}`;
        } else {
          return `L${value.start * svgWidth.value},${y}M${value.end * svgWidth.value},${y}`;
        }
      })
      .join(''),
    fill:
      segments.value
        .map(
          (value) =>
            `${value.start * svgWidth.value},${decibelsToY(value.volume)} ${value.end * svgWidth.value},${decibelsToY(value.volume)}`,
        )
        .join(' ') + ` ${svgWidth.value},${contentHeight.value} 0,${contentHeight.value}`,
  };
});

const clamp = (value: number, min: number, max: number): number => {
  return Math.max(min, Math.min(max, value));
};

const decibelsToY = (value: number): number => {
  return clamp((1 - Math.pow(10, value / 20)) * contentHeight.value, 0, contentHeight.value);
};

const YToDecibels = (value: number): number => {
  return clamp(Math.log10(1 - value / contentHeight.value) * 20, -60, 0);
};

const getSVGCoords = (e: MouseEvent) => {
  const svg = envelopeParent.value;
  if (svg == null) return { x: 0, y: 0 };
  const points = new DOMPoint(e.clientX, e.clientY);
  const CTM = svg.getScreenCTM();
  if (CTM == null) return { x: 0, y: 0 };
  const svgPoint = points.matrixTransform(CTM.inverse());
  return { x: clamp(svgPoint.x, 0, svgWidth.value), y: clamp(svgPoint.y, 0, contentHeight.value) };
};

const handlePointerDown = (
  e: PointerEvent,
  index: number,
  type: 'volume' | 'start' | 'end' | 'hstart' | 'hend',
) => {
  if (props.disabled) return;
  e.stopPropagation();
  if (type === 'start' && index === 0) return;
  if (type === 'end' && index === segments.value.length - 1) return;
  dragging.value = { index, type, dragged: false };
};

const handlePointerMove = (e: PointerEvent) => {
  if (dragging.value == null || props.disabled) return;
  dragging.value.dragged = true;
  const { x, y } = getSVGCoords(e);

  switch (dragging.value.type) {
    case 'volume': {
      const index = dragging.value.index;
      const current = segments.value[index];
      if (current == null) {
        dragging.value = null;
        return;
      }

      current.volume = YToDecibels(y);
      break;
    }
    case 'start': {
      const index = dragging.value.index;
      const current = segments.value[index];
      if (current == null) {
        dragging.value = null;
        return;
      }
      const prevSeg = segments.value[index - 1];

      const minX = prevSeg ? prevSeg.end + MIN_GAP : 0;
      const maxX = current.end - MIN_GAP;
      current.start = clamp(
        (x / svgWidth.value - timeRange.value.start) / timeRange.value.delta,
        minX,
        maxX,
      );
      break;
    }
    case 'end': {
      const index = dragging.value.index;
      const current = segments.value[index];
      if (current == null) {
        dragging.value = null;
        return;
      }
      const nextSeg = segments.value[index + 1];

      const minX = current.start + MIN_GAP;
      const maxX = nextSeg ? nextSeg.start - MIN_GAP : 1;
      current.end = clamp(
        (x / svgWidth.value - timeRange.value.start) / timeRange.value.delta,
        minX,
        maxX,
      );
      break;
    }
    case 'hstart': {
      timeRange.value.start = clamp(x / svgWidth.value, 0, timeRange.value.end);
      timeRange.value.delta = timeRange.value.end - timeRange.value.start;
      break;
    }
    case 'hend': {
      timeRange.value.end = clamp(x / svgWidth.value, timeRange.value.start, 1);
      timeRange.value.delta = timeRange.value.end - timeRange.value.start;
      break;
    }
  }
};

const handlePointerUp = () => {
  if (dragging.value != null) {
    if (!dragging.value.dragged && !dragging.value.type.startsWith('h')) {
      selectedIdx.value = dragging.value.index;
    } else {
      saveEditorValue();
    }
    dragging.value = null;
  }
};

useEventListener(document, 'pointermove', handlePointerMove);
useEventListener(document, 'pointerup', handlePointerUp);

const handleAddOrSplit = (svgX: number) => {
  if (props.disabled) return;
  if (!uiState.isEnvelopeVisible) return;
  if (segments.value.length === 0) {
    segments.value.push({ start: 0, end: 1, volume: 0.5 });
    saveEditorValue();
    return;
  }
  const targetIdx = segments.value.findIndex((s) => svgX >= s.start && svgX <= s.end);
  if (targetIdx !== -1) {
    const target = segments.value[targetIdx]!;
    const segmentWidth = target.end - target.start;

    if (segmentWidth > MIN_GAP * 2 + MIN_GAP) {
      const leftEnd = svgX - MIN_GAP / 2;
      const rightStart = svgX + MIN_GAP / 2;

      segments.value[targetIdx] = { ...target, end: leftEnd };
      segments.value.splice(targetIdx + 1, 0, {
        start: rightStart,
        end: target.end,
        volume: target.volume,
      });
      saveEditorValue();
      return;
    }
  }

  for (let i = 0; i < segments.value.length - 1; i++) {
    const gapStart = segments.value[i]!.end;
    const gapEnd = segments.value[i + 1]!.start;
    if (svgX > gapStart && svgX < gapEnd) {
      const availableWidth = gapEnd - gapStart;
      if (availableWidth > MIN_GAP + MIN_GAP * 2) {
        const s = svgX - 0.04;
        const e = svgX + 0.04;
        const finalS = Math.max(gapStart + MIN_GAP, s);
        const finalE = Math.min(gapEnd - MIN_GAP, e);
        segments.value.splice(i + 1, 0, { start: finalS, end: finalE, volume: 0.5 });
        saveEditorValue();
        return;
      }
    }
  }
};

const clearSegments = () => {
  if (props.disabled || !uiState.isEnvelopeVisible) return;
  segments.value = [];
  selectedIdx.value = null;
  saveEditorValue();
};

const addSegment = () => {
  if (props.disabled) return;
  uiState.isEnvelopeVisible = true;
  handleAddOrSplit(0.5);
  saveEditorValue();
};

const removeSegment = () => {
  if (props.disabled || !uiState.isEnvelopeVisible) return;
  if (selectedIdx.value != null) {
    segments.value.splice(selectedIdx.value, 1);
    if (selectedIdx.value === 0) {
      const first = segments.value[0];
      if (first != null) {
        first.start = 0;
      }
    } else if (selectedIdx.value === segments.value.length) {
      const last = segments.value[segments.value.length - 1];
      if (last != null) {
        last.end = 1;
      }
    }
    saveEditorValue();
  }
};

const seek = (event: MouseEvent) => {
  if (!props.disabled) return;
  if (selectedCue.value == null || event.button !== 0) {
    return;
  }
  const activeCue = showState.activeCues[selectedCue.value.id];
  if (activeCue == null) {
    return;
  }
  const position =
    (event.offsetX - timeRange.value.start * svgWidth.value) /
    (svgWidth.value * timeRange.value.delta);
  if (position > 0 && position < 1) {
    api.sendSeekTo(selectedCue.value.id, position * activeCue.duration);
  }
};

const skipFirstSilence = () => {
  if (selectedCue.value == null) {
    return;
  }
  const result = assetResult.get(selectedCue.value.id);
  if (result == null || result.metadata.duration == null) return;
  timeRange.value.start = clamp(
    (result.startTime || 0) / result.metadata.duration,
    0,
    timeRange.value.end,
  );
  timeRange.value.delta = timeRange.value.end - timeRange.value.start;
  saveEditorValue();
};

const skipLastSilence = () => {
  if (selectedCue.value == null) {
    return;
  }
  const result = assetResult.get(selectedCue.value.id);
  if (result == null || result.metadata.duration == null) return;
  timeRange.value.end = clamp(
    (result.endTime || 1) / result.metadata.duration,
    timeRange.value.start,
    1,
  );
  timeRange.value.delta = timeRange.value.end - timeRange.value.start;
  saveEditorValue();
};

const menuRef = useTemplateRef('menu');
const menuItems = computed(() => [
  {
    label: t('main.bottomEditor.timeLevels.scaleWaveform'),
    icon: uiState.scaleWaveform ? mdiCheckboxMarked : mdiCheckboxBlankOutline,
    command: () => (uiState.scaleWaveform = !uiState.scaleWaveform),
  },
]);
</script>

<template>
  <div
    class="flex flex-row gap-2"
    @contextmenu.prevent="menuRef?.show($event)"
  >
    <div class="mb-2 flex flex-col items-center justify-center gap-2">
      <div class="flex flex-row items-center gap-2">
        <time-input
          v-model="timeRange.start"
          width="175px"
          :label="t('main.bottomEditor.timeLevels.startTime')"
          :multiply="metadata?.duration || 1"
          :default-value="0"
          @update="saveEditorValue"
          @pointerdown.stop
        />
        <button-wrapper
          v-tooltip.right="t('main.bottomEditor.timeLevels.skipFirstSilence')"
          class="w-12"
          size="small"
          :icon="mdiSkipNext"
          :disabled="selectedCue != null && selectedCue.id in showState.activeCues"
          @click="skipFirstSilence"
        />
      </div>
      <div class="flex flex-row items-center gap-2">
        <time-input
          v-model="timeRange.end"
          width="175px"
          :label="t('main.bottomEditor.timeLevels.endTime')"
          :multiply="metadata?.duration || 1"
          :default-value="1"
          @update="saveEditorValue"
          @pointerdown.stop
        />
        <button-wrapper
          v-tooltip.right="t('main.bottomEditor.timeLevels.skipLastSilence')"
          class="w-12"
          size="small"
          :icon="mdiSkipPrevious"
          :disabled="selectedCue != null && selectedCue.id in showState.activeCues"
          @click="skipLastSilence"
        />
      </div>
      <button-wrapper
        :icon="uiState.isEnvelopeVisible ? mdiEye : mdiEyeOff"
        :label="t('main.bottomEditor.timeLevels.envelopeVisible')"
        size="small"
        class="self-start"
        width="175px"
        @click="uiState.isEnvelopeVisible = !uiState.isEnvelopeVisible"
      />
    </div>
    <div
      :style="{ height: `${props.heightPx}px` }"
      class="relative w-full border border-(--p-form-field-border-color)"
      ref="container"
    >
      <div
        ref="tooltip"
        v-show="!isOutside"
        v-if="selectedCue != null && svgRef != null && parent != null"
        class="pointer-events-none absolute top-0 left-0 rounded border border-(--p-form-field-border-color) bg-(--p-content-background) pr-1 pl-1"
      />
      <svg
        v-show="selectedCue != null"
        ref="svg"
        preserveAspectRatio="none"
        xmlns="http://www.w3.org/2000/svg"
        :viewBox="`0 0 ${svgWidth} ${contentHeight}`"
        width="100%"
        :height="`${contentHeight}px`"
        @dblclick="
          (e: MouseEvent) => {
            const { x } = getSVGCoords(e);
            handleAddOrSplit((x / svgWidth - timeRange.start) / timeRange.delta);
          }
        "
        @pointerdown="seek"
      >
        <rect
          :class="$style.waveform"
          x="0"
          :y="contentHeight / 2"
          height="1"
          :width="svgWidth"
        />
        <path
          v-if="waveformPath != null"
          :d="waveformPath"
          :transform="waveformTransform"
          :class="$style.waveform"
          transform-origin="center"
        />
        <rect
          :x="startPos"
          y="0"
          width="2"
          :height="contentHeight"
          fill="blue"
        />
        <rect
          :x="endPos"
          y="0"
          width="2"
          :height="contentHeight"
          fill="blue"
        />
        <rect
          :x="startPos - 10"
          y="0"
          width="20"
          :height="contentHeight"
          fill="transparent"
          :style="{ cursor: props.disabled ? '' : 'ew-resize' }"
          @pointerdown="handlePointerDown($event, 0, 'hstart')"
        />
        <rect
          :x="endPos - 10"
          y="0"
          width="20"
          :height="contentHeight"
          fill="transparent"
          :style="{ cursor: props.disabled ? '' : 'ew-resize' }"
          @pointerdown="handlePointerDown($event, 0, 'hend')"
        />
        <rect
          v-show="selectedCue != null && selectedCue.id in showState.activeCues"
          ref="position"
          x="0"
          y="0"
          width="2"
          :height="contentHeight"
          fill="yellow"
        />
        <g
          v-show="uiState.isEnvelopeVisible"
          ref="parent"
        >
          <path
            :d="linePath.dot"
            fill="none"
            style="stroke: var(--p-primary-500); stroke-width: 3px; stroke-dasharray: 8, 4"
            :transform="`translate(${timeRange.start * svgWidth}, 0) scale(${timeRange.delta},1)`"
          />
          <polyline
            :points="linePath.fill"
            style="fill: rgb(from var(--p-primary-500) r g b / 20%)"
            :transform="`translate(${timeRange.start * svgWidth}, 0) scale(${timeRange.delta},1)`"
          />
          <g
            v-for="(seg, i) in segments"
            :key="i"
            :class="{
              [$style['selected']]: selectedIdx == i,
              [$style['disabled']]: props.disabled,
            }"
          >
            <rect
              :x="(timeRange.start + seg.start * timeRange.delta) * svgWidth"
              :y="decibelsToY(seg.volume) - 2"
              :width="(seg.end - seg.start) * svgWidth * timeRange.delta"
              height="4"
              :class="$style['bar']"
            />
            <rect
              :x="(timeRange.start + seg.start * timeRange.delta) * svgWidth"
              :y="decibelsToY(seg.volume) - 10"
              :width="(seg.end - seg.start) * svgWidth * timeRange.delta"
              height="20"
              :style="{ cursor: props.disabled ? '' : 'ns-resize' }"
              fill="transparent"
              @pointerdown="handlePointerDown($event, i, 'volume')"
            />
            <circle
              :cx="(timeRange.start + seg.start * timeRange.delta) * svgWidth"
              :cy="decibelsToY(seg.volume)"
              :r="i == 0 || props.disabled ? 3 : 8"
              :class="i == 0 || props.disabled ? $style['handle-locked'] : $style['handle']"
              @pointerdown="handlePointerDown($event, i, 'start')"
            />
            <circle
              :cx="(timeRange.start + seg.end * timeRange.delta) * svgWidth"
              :cy="decibelsToY(seg.volume)"
              :r="i == segments.length - 1 || props.disabled ? 3 : 8"
              :class="
                i == segments.length - 1 || props.disabled
                  ? $style['handle-locked']
                  : $style['handle']
              "
              @pointerdown="handlePointerDown($event, i, 'end')"
            />
          </g>
        </g>
      </svg>
    </div>
    <div class="flex flex-col items-center justify-center gap-2">
      <button-wrapper
        rounded
        :icon="mdiPlus"
        severity="success"
        variant="outlined"
        size="small"
        :disabled="props.disabled"
        @click="addSegment"
      />
      <button-wrapper
        rounded
        :icon="mdiMinus"
        severity="danger"
        variant="outlined"
        size="small"
        :disabled="props.disabled || selectedIdx == null"
        @click="removeSegment"
      />
      <button-wrapper
        rounded
        :icon="mdiTrashCan"
        severity="secondary"
        variant="outlined"
        size="small"
        :disabled="props.disabled || !uiState.isEnvelopeVisible"
        @click="clearSegments"
      />
    </div>
    <context-menu
      ref="menu"
      :model="menuItems"
    >
      <template #itemicon="innerProps">
        <path-icon :icon="innerProps.item.icon || null" />
      </template>
    </context-menu>
  </div>
</template>

<style lang="css" module>
.waveform {
  stroke: rgb(from var(--p-surface-500) r g b / 0.8);
}
.disabled {
  .bar {
    fill: var(--p-surface-500);
  }
  .handle:hover {
    stroke: black;
  }
}
.selected {
  .bar {
    fill: var(--p-orange-500);
  }
  .handle {
    fill: var(--p-orange-500);
  }
}
.bar {
  fill: var(--p-primary-500);
  stroke: var(--p-surface-500);
  stroke-width: 1px;
}
.handle {
  stroke-width: 2px;
  cursor: ew-resize;
  fill: var(--p-primary-500);
}
.handle:hover {
  stroke: white;
}
.handle-locked {
  fill: #444;
}
</style>
