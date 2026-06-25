<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import {
  mdiArrowCollapseDown,
  mdiArrowExpandDown,
  mdiArrowRightBold,
  mdiDragVertical,
  mdiMenuDown,
  mdiMenuRight,
  mdiPause,
  mdiPlay,
  mdiRepeat,
  mdiUpload,
} from '@mdi/js';
import type { FlatCueEntry } from '../../stores/showModel';
import { computed, toRaw, useTemplateRef, watch } from 'vue';
import { useUiState } from '../../stores/uiState';
import { useShowState } from '../../stores/showState';
import {
  buildCueName,
  calculateDuration,
  formatToSeconds,
  getCueIcon,
  getLockCursorToSelection,
  secondsToFormat,
} from '../../utils';
import { useApi } from '../../api';
import { useAssetResult } from '../../stores/assetResult';
import type { PlaybackStatus } from '../../types/PlaybackStatus';
import { usePosition } from '../../composables/usePosition';
import PathIcon from '../display/PathIcon.vue';
import ProgressSpinnerWrapper from '../wrapper/ProgressSpinnerWrapper.vue';

const api = useApi();
const uiState = useUiState();
const showState = useShowState();
const assetResult = useAssetResult();

const props = defineProps<{
  item: FlatCueEntry;
  isDragOver: boolean;
}>();

const isSelected = computed(() => uiState.selectedRows.has(props.item.cue.id));
const isExpanded = computed(() => uiState.expandedRows.includes(props.item.cue.id));
const isPlaybackCursor = computed(() => showState.playbackCursor === props.item.cue.id);
const cueIcon = computed(() => getCueIcon(props.item.cue.params.type));

const preWaitRef = useTemplateRef('preWait');
const durationRef = useTemplateRef('duration');

usePosition((pos) => {
  if (
    preWaitRef.value == null ||
    durationRef.value == null ||
    preWaitRef.value.children.length < 2 ||
    durationRef.value.children.length < 2
  )
    return;
  const position = pos[props.item.cue.id];
  const activeCue = showState.activeCues[props.item.cue.id];
  if (position != null && activeCue != null && activeCue.duration > 0) {
    if (activeCue.status.startsWith('pre')) {
      if (durationRef.value.children[1]!.textContent !== durationText.value) {
        durationRef.value.children[1]!.textContent = durationText.value;
        (durationRef.value.children[0]! as HTMLElement).style.transform = 'scaleX(0)';
      }
      preWaitRef.value.children[1]!.textContent = secondsToFormat(
        uiState.preWaitDisplayMode === 'elapsed' ? position : activeCue.duration - position,
      );
      (preWaitRef.value.children[0]! as HTMLElement).style.transform =
        `scaleX(${position / activeCue.duration})`;
    } else {
      if (preWaitRef.value.children[1]!.textContent !== preWaitText) {
        preWaitRef.value.children[1]!.textContent = preWaitText;
        (preWaitRef.value.children[0]! as HTMLElement).style.transform = 'scaleX(0)';
      }
      durationRef.value.children[1]!.textContent = secondsToFormat(
        uiState.durationDisplayMode === 'elapsed' ? position : activeCue.duration - position,
      );
      (durationRef.value.children[0]! as HTMLElement).style.transform =
        `scaleX(${position / activeCue.duration})`;
    }
  } else {
    if (
      (preWaitRef.value.children[1]! as HTMLElement).contentEditable !== 'true' &&
      preWaitRef.value.children[1]!.textContent !== preWaitText
    ) {
      preWaitRef.value.children[1]!.textContent = preWaitText;
      (preWaitRef.value.children[0]! as HTMLElement).style.transform = 'scaleX(0)';
    }
    if (
      (durationRef.value.children[1]! as HTMLElement).contentEditable !== 'true' &&
      durationRef.value.children[1]!.textContent !== durationText.value
    ) {
      durationRef.value.children[1]!.textContent = durationText.value;
      (durationRef.value.children[0]! as HTMLElement).style.transform = 'scaleX(0)';
    }
  }
});

let preWaitText = '--:--.--';
watch(
  () => props.item.cue.preWait,
  () => {
    preWaitText = secondsToFormat(props.item.cue.preWait === 0.0 ? null : props.item.cue.preWait);
  },
  { immediate: true },
);

const durationText = computed(() => {
  return secondsToFormat(
    calculateDuration(props.item.cue.params, assetResult.getMetadata(props.item.cue.id)?.duration),
  );
});

const setPlaybackCursor = (cueId: string) => {
  if (!getLockCursorToSelection()) {
    api.setPlaybackCursor(cueId);
  }
};

const dragStart = (event: DragEvent) => {
  const targetId = props.item.cue.id;
  if (targetId && event.dataTransfer) {
    if (!uiState.selectedRows.has(targetId)) {
      uiState.setSelected(targetId);
    }
    event.dataTransfer.effectAllowed = 'move';
    event.dataTransfer.dropEffect = 'move';
  }
};

const drop = (event: DragEvent) => {
  event.preventDefault();
  if (event.dataTransfer) {
    const targetId = props.item.cue.id;
    if (targetId == null) return;
    api.moveCues(Array.from(uiState.selectedRows), { type: 'before', target: targetId });
  }
};

const openEditable = (e: MouseEvent, editType: string) => {
  if (uiState.mode !== 'edit') {
    return;
  }
  if (
    e.target == null ||
    !(e.target instanceof HTMLElement) ||
    e.target.contentEditable === 'true'
  ) {
    return;
  }
  if (props.item == null) return;
  if (editType === 'cuelist_duration') {
    const cueType = props.item.cue.params.type;
    if (cueType !== 'wait' && cueType !== 'fade') {
      return;
    }
  }
  if (editType === 'cuelist_post_wait') {
    if (props.item.isChainOverrided) {
      return;
    }
    if (props.item.chain.type !== 'afterStart') {
      return;
    }
  }
  e.target.contentEditable = 'true';
  e.target.classList.add('inEdit');
  e.target.dataset.prevText = e.target.innerText;
  const range = document.createRange();
  range.selectNodeContents(e.target);
  const sel = window.getSelection();
  if (sel != null) {
    sel.removeAllRanges();
    sel.addRange(range);
  } else {
    e.target.focus();
  }
};

const closeEditable = (target: EventTarget | null, needSave: boolean, editType: string) => {
  if (target == null || !(target instanceof HTMLElement) || target.contentEditable === 'false') {
    return;
  }
  target.contentEditable = 'false';
  target.classList.remove('inEdit');
  if (needSave) {
    if (props.item == null) return;
    const newCue = structuredClone(toRaw(props.item.cue));
    switch (editType) {
      case 'cuelist_number':
        newCue.number = target.innerText;
        break;
      case 'cuelist_name': {
        const newText = target.innerText.trim();
        if (newText === '') {
          newCue.name = null;
        } else {
          newCue.name = newText;
        }
        break;
      }
      case 'cuelist_pre_wait': {
        const newPreWait = formatToSeconds(target.innerText, false);
        newCue.preWait = newPreWait;
        break;
      }
      case 'cuelist_duration': {
        if (newCue.params.type === 'wait') {
          const newDuration = formatToSeconds(target.innerText, false);
          newCue.params.duration = newDuration;
        } else if (newCue.params.type === 'fade') {
          const newDuration = formatToSeconds(target.innerText, false);
          newCue.params.fadeParam.duration = newDuration;
        }
        break;
      }
    }
    api.updateCue(newCue);
  } else {
    if (target.dataset.prevText !== undefined) {
      target.innerText = target.dataset.prevText;
    }
  }
  delete target.dataset.prevText;
};

const status = computed(() => {
  const activeCue = showState.activeCues[props.item.cue.id];
  return activeCue != null ? activeCue.status : null;
});

const isStatusIn = (statusList: PlaybackStatus[]): boolean => {
  return status.value != null ? statusList.includes(status.value) : false;
};

const isPreWaitActive = computed(() => {
  return (
    props.item.cue.id in showState.activeCues &&
    showState.activeCues[props.item.cue.id]!.status.startsWith('pre')
  );
});

const isActive = computed((): boolean => {
  return (
    props.item.cue.id in showState.activeCues &&
    (['playing', 'paused', 'stopping', 'completed'] as PlaybackStatus[]).includes(
      showState.activeCues[props.item.cue.id]!.status,
    )
  );
});
</script>

<template>
  <tr
    :class="[
      isDragOver ? $style['drag-over-row'] : '',
      isSelected ? $style['selected-row'] : '',
      $style['cue-row'],
    ]"
    :data-cue-color="item.cue.color"
    @drop="drop"
  >
    <td
      headers="cuelist_handle"
      class="px-0"
      :class="uiState.mode == 'edit' ? 'cursor-grab' : ''"
      :draggable="uiState.mode == 'edit' ? 'true' : 'false'"
      @dragstart="dragStart"
      @pointerdown="
        (e) => {
          if (uiState.mode == 'edit') {
            e.stopPropagation();
          }
        }
      "
    >
      <path-icon
        v-show="uiState.mode == 'edit'"
        :icon="mdiDragVertical"
      />
    </td>
    <td
      headers="cuelist_cursor"
      style="padding-left: 12px; padding-right: 0px"
    >
      <path-icon
        class="cursor-pointer"
        :icon="isPlaybackCursor ? mdiArrowRightBold : null"
        @click="setPlaybackCursor(props.item.cue.id)"
      />
    </td>
    <td
      headers="cuelist_status"
      style="padding-left: 6px"
    >
      <path-icon
        v-show="isStatusIn(['playing', 'preWaiting'])"
        :icon="mdiPlay"
        color="success"
      />
      <path-icon
        v-show="isStatusIn(['paused', 'preWaitPaused'])"
        :icon="mdiPause"
        color="warning"
      />
      <path-icon
        v-show="status == 'loaded'"
        :icon="mdiUpload"
        color="warning"
      />
      <progress-spinner-wrapper
        v-show="status == 'stopping'"
        size="16px"
        color="warning"
      />
    </td>
    <td
      headers="cuelist_type"
      class="text-center"
      style="padding: 0px"
    >
      <path-icon :icon="cueIcon" />
    </td>
    <td
      headers="cuelist_number"
      class="text-center"
      @dblclick="openEditable($event, 'cuelist_number')"
      @blur="closeEditable($event.target, true, 'cuelist_number')"
      @keydown.enter.stop="closeEditable($event.target, true, 'cuelist_number')"
      @keydown.esc.stop="closeEditable($event.target, false, 'cuelist_number')"
    >
      {{ item.cue.number }}
    </td>
    <td
      headers="cuelist_name"
      class="overflow-hidden whitespace-nowrap"
      :style="{
        paddingLeft: `${item.level}em`,
      }"
      @dblclick="openEditable($event, 'cuelist_name')"
      @blur="closeEditable($event.target, true, 'cuelist_name')"
      @keydown.enter.stop="closeEditable($event.target, true, 'cuelist_name')"
      @keydown.esc.stop="closeEditable($event.target, false, 'cuelist_name')"
    >
      <path-icon
        :icon="item.isGroup ? (isExpanded ? mdiMenuDown : mdiMenuRight) : null"
        :tabindex="item.isGroup ? 0 : -1"
        :class="item.isGroup ? 'cursor-pointer' : ''"
        @click.stop="if (item.isGroup) uiState.toggleExpand(item.cue.id);"
        @pointerdown="item.isGroup && $event.stopPropagation()"
      />
      {{ item.cue.name != null ? item.cue.name : buildCueName(item.cue) }}
    </td>
    <td
      headers="cuelist_pre_wait"
      class="text-center"
      style="padding: 4px 4px"
    >
      <div
        ref="preWait"
        class="relative w-full h-3/4"
        :class="[isPreWaitActive ? 'border border-primary' : '']"
      >
        <div
          class="top-0 left-0 w-full h-full"
          style="
            transform-origin: left;
            background-color: rgb(from var(--p-primary-color) r g b / 0.5);
            transform: scaleX(0);
          "
        />
        <div
          class="absolute left-0 w-full text-center"
          style="top: 50%; transform: translateY(-50%)"
          @dblclick="if (!isPreWaitActive) openEditable($event, 'cuelist_pre_wait');"
          @blur="closeEditable($event.target, true, 'cuelist_pre_wait')"
          @keydown.enter.stop="closeEditable($event.target, true, 'cuelist_pre_wait')"
          @keydown.esc.stop="closeEditable($event.target, false, 'cuelist_pre_wait')"
        />
      </div>
    </td>
    <td
      headers="cuelist_duration"
      class="text-center"
      style="padding: 0px 4px"
    >
      <div
        ref="duration"
        class="relative w-full h-3/4"
        :class="[isActive ? 'border border-primary' : '']"
      >
        <div
          class="top-0 left-0 w-full h-full"
          style="
            transform-origin: left;
            background-color: rgb(from var(--p-primary-color) r g b / 0.5);
            transform: scaleX(0);
          "
        />
        <div
          class="absolute left-0 w-full text-center"
          style="top: 50%; transform: translateY(-50%)"
          @dblclick="if (!isActive) openEditable($event, 'cuelist_duration');"
          @blur="closeEditable($event.target, true, 'cuelist_duration')"
          @keydown.enter.stop="closeEditable($event.target, true, 'cuelist_duration')"
          @keydown.esc.stop="closeEditable($event.target, false, 'cuelist_duration')"
        />
      </div>
    </td>
    <td headers="cuelist_repeat">
      <path-icon
        v-show="
          (item.cue.params.type == 'audio' && item.cue.params.repeat) ||
          (item.cue.params.type == 'group' &&
            item.cue.params.mode.type == 'playlist' &&
            item.cue.params.mode.repeat)
        "
        :icon="mdiRepeat"
      />
    </td>
    <td headers="cuelist_chain">
      <path-icon
        v-show="item.chain.type == 'afterComplete'"
        :icon="mdiArrowCollapseDown"
      />
      <path-icon
        v-show="item.chain.type == 'afterStart'"
        :icon="mdiArrowExpandDown"
      />
    </td>
  </tr>
</template>

<style lang="css" module>
.selected-row {
  background-color: rgb(from var(--p-primary-color) r g b / 0.3);
}

.cue-row td {
  position: relative;
}

.cue-row td::after {
  content: '';
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 2px;
  opacity: 0;
  pointer-events: none;
  background-color: var(--p-primary-color);
}

.cue-row.drag-over-row td:after {
  opacity: 1;
}

.cue-row[data-cue-color='red'] {
  --row-color: 244 67 54;
}

.cue-row[data-cue-color='purple'] {
  --row-color: 156 39 176;
}

.cue-row[data-cue-color='blue'] {
  --row-color: 33 150 243;
}

.cue-row[data-cue-color='cyan'] {
  --row-color: 0 188 212;
}

.cue-row[data-cue-color='green'] {
  --row-color: 76 175 80;
}

.cue-row[data-cue-color='yellow'] {
  --row-color: 255 235 59;
}

.cue-row[data-cue-color='orange'] {
  --row-color: 255 152 0;
}

.cue-row[data-cue-color='grey'] {
  --row-color: 158 158 158;
}

.cue-row:not([data-cue-color='none']) {
  background-color: rgb(var(--row-color) / 0.2);
}

.cue-row:not([data-cue-color='none']) > td:nth-child(1) {
  background-color: rgb(var(--row-color) / 0.5);
}
</style>
