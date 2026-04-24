<template>
  <div class="d-flex flex-row ga-2">
    <div class="d-flex flex-column align-center justify-center ga-2 mb-2">
      <div class="d-flex flex-row align-center ga-2">
        <time-input
          v-model="timeRange.start"
          width="175px"
          :label="t('main.bottomEditor.timeLevels.startTime')"
          :multiply="metadata?.duration || 1"
          @update="emit('update')"
          @pointerdown.stop
        />
        <v-tooltip target="cursor">
          <template #activator="{ props: activatorProps }">
            <v-btn
              v-bind="activatorProps"
              density="compact"
              variant="outlined"
              :icon="mdiSkipNext"
              :disabled="selectedCue != null && selectedCue.id in showState.activeCues"
              @click="skipFirstSilence"
            />
          </template>
          <span>{{ t('main.bottomEditor.timeLevels.skipFirstSilence') }}</span>
        </v-tooltip>
      </div>
      <div class="d-flex flex-row align-center ga-2">
        <time-input
          v-model="timeRange.end"
          width="175px"
          :label="t('main.bottomEditor.timeLevels.endTime')"
          :multiply="metadata?.duration || 1"
          @update="emit('update')"
          @pointerdown.stop
        />
        <v-tooltip target="cursor">
          <template #activator="{ props: activatorProps }">
            <v-btn
              v-bind="activatorProps"
              density="compact"
              variant="outlined"
              :icon="mdiSkipPrevious"
              :disabled="selectedCue != null && selectedCue.id in showState.activeCues"
              @click="skipLastSilence"
            />
          </template>
          <span>{{ t('main.bottomEditor.timeLevels.skipLastSilence') }}</span>
        </v-tooltip>
      </div>
      <v-btn
        :prepend-icon="uiState.isEnvelopeVisible ? mdiEye : mdiEyeOff"
        class="align-self-start"
        density="compact"
        color="white"
        variant="outlined"
        width="175px"
        @click="uiState.isEnvelopeVisible = !uiState.isEnvelopeVisible"
      >
        {{ t('main.bottomEditor.timeLevels.envelopeVisible') }}
      </v-btn>
    </div>
    <div
      :style="{ height: `${props.heightPx}px` }"
      class="w-100 border-md"
    >
      <v-sheet
        v-show="!isOutside"
        v-if="selectedCue != null && svgRef != null && parent != null"
        style="position: absolute; top: 0px; left: 0px;"
        :style="tooltipStyle"
        class="pl-1 pr-1 rounded text-caption"
      >
        {{ tooltipText }}
      </v-sheet>
      <svg
        v-show="selectedCue != null"
        ref="svg"
        preserveAspectRatio="none"
        xmlns="http://www.w3.org/2000/svg"
        :viewBox="`0 0 ${svgWidth} ${contentHeight}`"
        width="100%"
        :height="`${contentHeight}px`"
        @dblclick="(e: MouseEvent) => {
          const {x} = getSVGCoords(e);
          handleAddOrSplit(((x / svgWidth) - timeRange.start) / timeRange.delta);
        }"
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
          :transform="`translate(0, ${contentHeight * 0.125})`"
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
          :style="{cursor: props.disabled ? '' : 'ew-resize'}"
          @pointerdown="handlePointerDown($event, 0, 'hstart')"
        />
        <rect
          :x="endPos - 10"
          y="0"
          width="20"
          :height="contentHeight"
          fill="transparent"
          :style="{cursor: props.disabled ? '' : 'ew-resize'}"
          @pointerdown="handlePointerDown($event, 0, 'hend')"
        />
        <rect
          v-show="position != 0"
          :style="playCursorStyle"
          x="0"
          y="0"
          width="2"
          :height="contentHeight"
          fill="yellow"
        />
        <g
          ref="parent"
          v-show="uiState.isEnvelopeVisible"
        >
          <path
            :d="linePath.dot"
            fill="none"
            style="stroke: rgb(var(--v-theme-primary)); stroke-width: 3px; stroke-dasharray: 8, 4;"
            :transform="`translate(${timeRange.start * svgWidth}, 0) scale(${timeRange.delta},1)`"
          />
          <polyline
            :points="linePath.fill"
            style="fill: rgb(var(--v-theme-primary), 20%)"
            :transform="`translate(${timeRange.start * svgWidth}, 0) scale(${timeRange.delta},1)`"
          />
          <g
            v-for="(seg, i) in segments"
            :key="i"
            :class="{[$style['selected']]: selectedIdx == i}"
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
              :style="{cursor: props.disabled ? '' : 'ns-resize'}"
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
              :r="i == (segments.length - 1) || props.disabled ? 3 : 8"
              :class="i == (segments.length - 1) || props.disabled ? $style['handle-locked'] : $style['handle']"
              @pointerdown="handlePointerDown($event, i, 'end')"
            />
          </g>
        </g>
      </svg>
    </div>
    <div class="d-flex flex-column ga-2 align-center justify-center">
      <v-btn
        :icon="mdiPlus"
        density="compact"
        color="success"
        variant="outlined"
        :disabled="props.disabled"
        @click="addSegment"
      />
      <v-btn
        :icon="mdiMinus"
        density="compact"
        color="error"
        variant="outlined"
        :disabled="props.disabled || selectedIdx == null"
        @click="removeSegment"
      />
      <v-btn
        :icon="mdiTrashCan"
        density="compact"
        color="white"
        variant="outlined"
        :disabled="props.disabled || !uiState.isEnvelopeVisible"
        @click="clearSegments"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, shallowRef, StyleValue, toRaw, useTemplateRef, watch } from 'vue';
import { useAssetResult } from '../../stores/assetResult';
import { useShowState } from '../../stores/showstate';
import { useElementSize, useEventListener, useMouseInElement, useParentElement, useWebWorkerFn, watchDebounced } from '@vueuse/core';
import { secondsToFormat } from '../../utils';
import { Cue } from '../../types/Cue';
import { mdiEye, mdiEyeOff, mdiMinus, mdiPlus, mdiSkipNext, mdiSkipPrevious, mdiTrashCan } from '@mdi/js';
import { useApi } from '../../api';
import { useI18n } from 'vue-i18n';
import TimeInput from '../input/TimeInput.vue';
import { useUiState } from '../../stores/uistate';

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
    disabled?: boolean;
  }>(),
  {
    heightPx: 75,
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
  let result = [...seg].sort((a, b) => a.start - b.start);
  if (result.length > 0) {
    result[0]!.start = 0;
    result[result.length - 1]!.end = 1;
  }
  return result;
};

const buildTimeRange = () => {
  const duration = metadata.value?.duration || 1;
  const start = selectedCue.value?.params.type == 'audio' ? (selectedCue.value.params.startTime || 0) / duration : 0;
  const end = selectedCue.value?.params.type == 'audio' ? (selectedCue.value.params.endTime || duration) / duration : 1;
  return { start, end, delta: end - start };
};

const dragging = ref<{
  index: number;
  type: 'volume' | 'start' | 'end' | 'hstart' | 'hend';
  dragged: boolean;
} | null>(null);
const selectedIdx = ref<number | null>(null);
const segments = ref<Segment[]>(selectedCue.value != null && selectedCue.value.params.type == 'audio' ? normSegments(selectedCue.value.params.envelope) : []);

watch(selectedCue, (newCue, oldCue) => {
  if (newCue?.id != oldCue?.id || (selectedIdx.value != null && newCue?.params.type == 'audio' && newCue.params.envelope.length <= selectedIdx.value)) {
    selectedIdx.value = null;
  }
  dragging.value = null;
  segments.value = selectedCue.value != null && selectedCue.value.params.type == 'audio' ? normSegments(selectedCue.value.params.envelope) : [];
  timeRange.value = buildTimeRange();
});

const contentHeight = computed(() => props.heightPx - 4);
const metadata = computed(() => (selectedCue.value ? assetResult.getMetadata(selectedCue.value.id) : null));

const timeRange = ref<{
  start: number;
  end: number;
  delta: number;
}>(buildTimeRange());

const startPos = computed<number>(() => timeRange.value.start * (svgWidth.value - 1));
const endPos = computed<number>(() => timeRange.value.end * (svgWidth.value - 1) - 1);

const svgRef = useTemplateRef('svg');
const { width: svgWidth } = useElementSize(svgRef);
const parent = useParentElement();
const position = computed<number>(() => {
  if (selectedCue.value == null) return 0;
  const activeCue = showState.activeCues[selectedCue.value.id];
  if (activeCue != null && activeCue.duration !== 0) {
    return activeCue.position / activeCue.duration;
  } else {
    return 0;
  }
});

const playCursorStyle = computed(() => {
  const range = timeRange.value;
  const x = (range.start + position.value * range.delta) * (svgWidth.value - 1);
  return {
    transform: `translateX(${x}px)`,
    transition: 'transform 10ms linear',
  };
});

const waveformPath = shallowRef('');

const buildWaveformPath = (source: number[], height: number, width: number) => {
  let result = '';
  const amp = height * 0.375;

  const samplePerPixel = source.length / width;
  for (let i = 0; i < width; i++) {
    let start = Math.floor(i * samplePerPixel);
    let end = Math.floor((i + 1) * samplePerPixel);

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

const { workerFn, workerStatus } = useWebWorkerFn(buildWaveformPath);

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
  if (workerStatus.value != 'RUNNING') {
    waveformPath.value = await workerFn(toRaw(source), contentHeight.value, svgWidth.value);
  }
};

watchDebounced([svgWidth, contentHeight, () => assetResult.get(selectedCue.value?.id)?.waveform], updateWaveformPath, { debounce: 200 });

const {
  x: mouseX,
  y: mouseY,
  elementX,
  isOutside,
} = useMouseInElement(svgRef, { handleOutside: false, touch: false });

const tooltipText = computed(() => {
  if (selectedCue.value == null) {
    return '--:--.-- / --:--.--';
  }
  const duration = assetResult.getMetadata(selectedCue.value.id)?.duration;
  if (duration == null) {
    return '--:--.-- / --:--.--';
  }
  return `${secondsToFormat((elementX.value / svgWidth.value) * duration)} / ${secondsToFormat(duration)}`;
});

const tooltipStyle = computed<StyleValue>(() => {
  if (parent.value == null) return {};
  const parentRect = parent.value.getBoundingClientRect();
  return {
    transform: `translateX(${parentRect.right - mouseX.value > 180 ? mouseX.value - parentRect.left + 15 : mouseX.value - parentRect.left - 150}px) translateY(${mouseY.value - parentRect.top - 10}px)`,
  };
});

const saveEditorValue = () => {
  if (props.disabled) return;
  if (selectedCue.value?.params.type != 'audio') return;
  selectedCue.value.params.envelope = segments.value;

  const duration = metadata.value?.duration || 1;
  selectedCue.value.params.startTime = timeRange.value.start == 0 ? null : timeRange.value.start * duration;
  selectedCue.value.params.endTime = timeRange.value.end == 1 ? null : timeRange.value.end * duration;
  emit('update');
};

const linePath = computed<{
  dot: string;
  fill: string;
}>(() => {
  if (segments.value.length == 0) return {
    dot: '',
    fill: '',
  };

  return {
    dot: segments.value.map((value, i) => {
      const y = decibelsToY(value.volume);
      if (i == 0) {
        return `M${value.end * svgWidth.value},${y}`;
      } else if (i == segments.value.length - 1) {
        return `L${value.start * svgWidth.value},${y}`;
      } else {
        return `L${value.start * svgWidth.value},${y}M${value.end * svgWidth.value},${y}`;
      }
    }).join(''),
    fill: segments.value.map(value => `${value.start * svgWidth.value},${decibelsToY(value.volume)} ${value.end * svgWidth.value},${decibelsToY(value.volume)}`).join(' ') + ` ${svgWidth.value},${contentHeight.value} 0,${contentHeight.value}`,
  };
});

const clamp = (value: number, min: number, max: number): number => {
  return Math.max(min, Math.min(max, value));
};

const decibelsToY = (value: number): number => {
  return clamp((1 - Math.pow(10, value / 20)) * contentHeight.value, 0, contentHeight.value);
};

const YToDecibels = (value: number): number => {
  return clamp(Math.log10(1 - (value / contentHeight.value)) * 20, -60, 0);
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

const handlePointerDown = (e: PointerEvent, index: number, type: 'volume' | 'start' | 'end' | 'hstart' | 'hend') => {
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
      current.start = clamp(((x / svgWidth.value) - timeRange.value.start) / timeRange.value.delta, minX, maxX);
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
      current.end = clamp(((x / svgWidth.value) - timeRange.value.start) / timeRange.value.delta, minX, maxX);
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
  if (segments.value.length == 0) {
    segments.value.push({ start: 0, end: 1, volume: 0.5 });
    saveEditorValue();
    return;
  }
  const targetIdx = segments.value.findIndex(s => svgX >= s.start && svgX <= s.end);
  if (targetIdx !== -1) {
    const target = segments.value[targetIdx]!;
    const segmentWidth = target.end - target.start;

    if (segmentWidth > (MIN_GAP * 2) + MIN_GAP) {
      const leftEnd = svgX - (MIN_GAP / 2);
      const rightStart = svgX + (MIN_GAP / 2);

      segments.value[targetIdx] = { ...target, end: leftEnd };
      segments.value.splice(targetIdx + 1, 0, { start: rightStart, end: target.end, volume: target.volume });
      saveEditorValue();
      return;
    }
  }

  for (let i = 0; i < segments.value.length - 1; i++) {
    const gapStart = segments.value[i]!.end;
    const gapEnd = segments.value[i + 1]!.start;
    if (svgX > gapStart && svgX < gapEnd) {
      const availableWidth = gapEnd - gapStart;
      if (availableWidth > MIN_GAP + (MIN_GAP * 2)) {
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
    if (selectedIdx.value == 0) {
      const first = segments.value[0];
      if (first != null) {
        first.start = 0;
      }
    } else if (selectedIdx.value == segments.value.length) {
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
  if (selectedCue.value == null || event.button != 0) {
    return;
  }
  const activeCue = showState.activeCues[selectedCue.value.id];
  if (activeCue == null) {
    return;
  }
  const position
    = (event.offsetX - timeRange.value.start * svgWidth.value)
      / (svgWidth.value * (timeRange.value.delta));
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
  timeRange.value.start = clamp((result.startTime || 0) / result.metadata.duration, 0, timeRange.value.end);
  timeRange.value.delta = timeRange.value.end - timeRange.value.start;
  saveEditorValue();
};

const skipLastSilence = () => {
  if (selectedCue.value == null) {
    return;
  }
  const result = assetResult.get(selectedCue.value.id);
  if (result == null || result.metadata.duration == null) return;
  timeRange.value.end = clamp((result.endTime || 1) / result.metadata.duration, timeRange.value.start, 1);
  timeRange.value.delta = timeRange.value.end - timeRange.value.start;
  saveEditorValue();
};
</script>

<style lang="css" module>
  .waveform {
    stroke: rgb(var(--v-theme-surface-variant), 0.8);
  }
  .disabled {
    .bar {
      fill: rgb(var(--v-theme-surface))
    }
    .handle:hover {
      stroke: black;
    }
  }
  .selected {
    .bar {
      fill: rgb(var(--v-theme-warning));
    }
    .handle {
      fill: rgb(var(--v-theme-warning));
    }
  }
  .bar {
    fill: rgb(var(--v-theme-primary));
    stroke: rgb(var(--v-theme-surface-variant));
    stroke-width: 1px;
  }
  .handle {
    stroke-width: 2px;
    cursor: ew-resize;
    fill: rgb(var(--v-theme-primary));
  }
  .handle:hover {
    stroke: white;
  }
  .handle-locked {
    fill: #444;
  }
</style>
