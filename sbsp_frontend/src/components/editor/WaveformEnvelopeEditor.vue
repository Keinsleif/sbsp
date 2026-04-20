<template>
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
        handleAddOrSplit(x);
      }"
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
      >
        <path
          :d="linePath.dot"
          fill="none"
          style="stroke: rgb(var(--v-theme-primary)); stroke-width: 3px; stroke-dasharray: 8, 4;"
          :transform="`translate(${timeRange.start}, 0) scale(${timeRange.delta},1)`"
        />
        <polyline
          :points="linePath.fill"
          style="fill: rgb(var(--v-theme-primary), 20%)"
          :transform="`translate(${timeRange.start}, 0) scale(${timeRange.delta},1)`"
        />
        <g
          v-for="(seg, i) in segments"
          :key="i"
        >
          <path
            :d="`M${seg.start * svgWidth},${decibelsToY(seg.volume)}H${seg.end * svgWidth}`"
            :style="{
              stroke: selectedIdx == i ? 'rgb(var(--v-theme-warning))' : 'rgb(var(--v-theme-primary))',
            }"
            stroke-width="4px"
          />
          <rect
            :x="(timeRange.start + seg.start) * svgWidth / timeRange.delta"
            :y="decibelsToY(seg.volume) - 10"
            :width="(seg.end - seg.start) * svgWidth / timeRange.delta"
            height="20"
            style="cursor: ns-resize;"
            fill="transparent"
            @pointerdown="handlePointerDown($event, i, 'volume')"
          />
          <circle
            :cx="(timeRange.start + seg.start) * svgWidth / timeRange.delta"
            :cy="decibelsToY(seg.volume)"
            :r="i == 0 ? 3 : 8"
            :class="i == 0 ? $style['handle-locked'] : $style['handle']"
            @pointerdown="handlePointerDown($event, i, 'start')"
          />
          <circle
            :cx="(timeRange.start + seg.end) * svgWidth / timeRange.delta"
            :cy="decibelsToY(seg.volume)"
            :r="i == (segments.length - 1) ? 3 : 8"
            :class="i == (segments.length - 1) ? $style['handle-locked'] : $style['handle']"
            @pointerdown="handlePointerDown($event, i, 'end')"
          />
        </g>
      </g>
    </svg>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, shallowRef, StyleValue, toRaw, useTemplateRef, watch } from 'vue';
import { useAssetResult } from '../../stores/assetResult';
import { useShowState } from '../../stores/showstate';
import { useElementSize, useEventListener, useMouseInElement, useParentElement, useWebWorkerFn, watchDebounced } from '@vueuse/core';
import { secondsToFormat } from '../../utils';
import { Cue } from '../../types/Cue';

const showState = useShowState();
const assetResult = useAssetResult();

const selectedCue = defineModel<Cue | null>();
const emit = defineEmits(['update']);
const envelopeParent = useTemplateRef('parent');
const props = withDefaults(
  defineProps<{
    startTime?: number | null;
    endTime?: number | null;
    heightPx?: number;
  }>(),
  {
    startTime: 0,
    endTime: 1,
    heightPx: 75,
  },
);

const contentHeight = computed(() => props.heightPx - 4);

const metadata = computed(() => (selectedCue.value ? assetResult.getMetadata(selectedCue.value.id) : null));
const timeRange = computed<{
  start: number;
  end: number;
  delta: number;
}>(() => {
  const duration = metadata.value?.duration || 1;
  const start = props.startTime != null && duration != null ? props.startTime / duration : 0;
  const end = props.endTime != null && duration != null ? props.endTime / duration : 1;
  return { start, end, delta: end - start };
});

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
  if (selectedCue.value?.params.type != 'audio') return;
  selectedCue.value.params.envelope = segments.value;
  emit('update');
};

const MIN_GAP = 0.005;

type Segment = {
  start: number;
  end: number;
  volume: number;
};

const segments = ref<Segment[]>(selectedCue.value != null && selectedCue.value.params.type == 'audio' ? [...selectedCue.value.params.envelope].sort((a, b) => a.start - b.start) : []);
watch(selectedCue, () => segments.value = selectedCue.value != null && selectedCue.value.params.type == 'audio' ? [...selectedCue.value.params.envelope].sort((a, b) => a.start - b.start) : []);
const dragging = ref<{
  index: number;
  type: 'volume' | 'start' | 'end';
  dragged: boolean;
} | null>(null);
const selectedIdx = ref<number | null>(null);

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

const handlePointerDown = (e: PointerEvent, index: number, type: 'volume' | 'start' | 'end') => {
  e.stopPropagation();
  if (type === 'start' && index === 0) return;
  if (type === 'end' && index === segments.value.length - 1) return;
  dragging.value = { index, type, dragged: false };
};

const handlePointerMove = (e: PointerEvent) => {
  if (dragging.value == null) return;
  dragging.value.dragged = true;
  const { x, y } = getSVGCoords(e);

  const index = dragging.value.index;
  const current = segments.value[index];
  if (current == null) {
    dragging.value = null;
    return;
  }
  const prevSeg = segments.value[index - 1];
  const nextSeg = segments.value[index + 1];

  switch (dragging.value.type) {
    case 'volume':
      current.volume = YToDecibels(y);
      break;
    case 'start': {
      const minX = prevSeg ? prevSeg.end + MIN_GAP : 0;
      const maxX = current.end - MIN_GAP;
      current.start = clamp(((x / svgWidth.value) - timeRange.value.start) / timeRange.value.delta, minX, maxX);
      break;
    }
    case 'end': {
      const minX = current.start + MIN_GAP;
      const maxX = nextSeg ? nextSeg.start - MIN_GAP : 1;
      current.end = clamp(((x / svgWidth.value) - timeRange.value.start) / timeRange.value.delta, minX, maxX);
      break;
    }
  }
};

const handlePointerUp = () => {
  if (dragging.value != null) {
    if (!dragging.value.dragged) {
      selectedIdx.value = dragging.value.index;
    } else {
      saveEditorValue();
    }
    dragging.value = null;
  }
};

useEventListener(document, 'pointermove', handlePointerMove);
useEventListener(document, 'pointerup', handlePointerUp);

const handleAddOrSplit = (clickX: number) => {
  const svgX = clickX / svgWidth.value;
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

  for (let i = 0; i < segments.value.length - 2; i++) {
    const gapStart = segments.value[i]!.end;
    const gapEnd = segments.value[i + 1]!.start;
    if (svgX > gapStart && svgX < gapEnd) {
      const availableWidth = gapEnd - gapStart;
      if (availableWidth > MIN_GAP + (MIN_GAP * 2)) {
        const s = svgX - 0.05;
        const e = svgX + 0.05;
        const finalS = Math.max(gapStart + MIN_GAP, s);
        const finalE = Math.min(gapEnd - MIN_GAP, e);
        segments.value.splice(i + 1, 0, { start: finalS, end: finalE, volume: 0.5 });
        saveEditorValue();
        break;
      }
    }
  }
};
</script>

<style lang="css" module>
  .waveform {
    stroke: rgb(var(--v-theme-surface-variant), 0.8);
  }
  .handle {
    cursor: ew-resize;
    fill: rgb(var(--v-theme-primary));
    :hover {
      fill: rgb(var(--v-theme-primary), 50%);
    }
  }
  .handle-locked {
    fill: #444;
  }
</style>
