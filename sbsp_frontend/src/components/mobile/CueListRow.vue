<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import {
  mdiArrowCollapseDown,
  mdiArrowExpandDown,
  mdiArrowRightBold,
  mdiMenuDown,
  mdiMenuRight,
  mdiPause,
  mdiPlay,
  mdiRepeat,
  mdiUpload,
} from '@mdi/js';
import type { FlatCueEntry } from '../../stores/showModel';
import { computed, useTemplateRef } from 'vue';
import { useUiState } from '../../stores/uiState';
import { useShowState } from '../../stores/showState';
import { buildCueName, getCueIcon } from '../../utils';
import type { PlaybackStatus } from '../../types/PlaybackStatus';
import { usePosition } from '../../composables/usePosition';
import PathIcon from '../display/PathIcon.vue';
import ProgressSpinnerWrapper from '../wrapper/ProgressSpinnerWrapper.vue';

const uiState = useUiState();
const showState = useShowState();

const props = defineProps<{
  item: FlatCueEntry;
}>();

const isExpanded = computed(() => uiState.expandedRows.includes(props.item.cue.id));
const isPlaybackCursor = computed(() => showState.playbackCursor === props.item.cue.id);
const cueIcon = computed(() => getCueIcon(props.item.cue.params.type));

const status = computed(() => {
  const activeCue = showState.activeCues[props.item.cue.id];
  return activeCue != null ? activeCue.status : null;
});

const rowRef = useTemplateRef('row');
usePosition((pos) => {
  if (rowRef.value == null) return;
  if (props.item.isHidden) return;
  const position = pos[props.item.cue.id];
  const activeCue = showState.activeCues[props.item.cue.id];
  if (activeCue == null || position == null || activeCue.duration === 0) {
    if (rowRef.value.style.background !== '') {
      rowRef.value.style.background = '';
    }
    return;
  }
  rowRef.value.style.background =
    (activeCue.status.startsWith('pre')
      ? 'linear-gradient(to right, rgb(from var(--p-orange-500) r g b / 0.5) '
      : 'linear-gradient(to right, rgb(from var(--p-primary-color) r g b / 0.5) ') +
    (position * 100) / activeCue.duration +
    '%, transparent ' +
    (position * 100) / activeCue.duration +
    '%) no-repeat';
});

const isStatusIn = (statusList: PlaybackStatus[]): boolean => {
  return status.value != null ? statusList.includes(status.value) : false;
};
</script>

<template>
  <tr ref="row">
    <td
      headers="cuelist_cursor"
      class="px-1 leading-none"
    >
      <path-icon :icon="isPlaybackCursor ? mdiArrowRightBold : null" />
    </td>
    <td
      headers="cuelist_status"
      class="px-1 leading-none"
    >
      <path-icon
        v-show="isStatusIn(['playing', 'preWaiting'])"
        :icon="mdiPlay"
        class="text-green-500"
      />
      <path-icon
        v-show="isStatusIn(['paused', 'preWaitPaused'])"
        :icon="mdiPause"
        class="text-orange-500"
      />
      <path-icon
        v-show="status == 'loaded'"
        :icon="mdiUpload"
        class="text-orange-500"
      />
      <progress-spinner-wrapper
        v-show="status == 'stopping'"
        size="16px"
        color="orange.500"
      />
    </td>
    <td
      headers="cuelist_type"
      class="text-center p-0 leading-none"
    >
      <path-icon :icon="cueIcon" />
    </td>
    <td
      headers="cuelist_number"
      class="border-x border-(--p-form-field-border-color) px-1 text-center"
    >
      {{ item.cue.number }}
    </td>
    <td
      headers="cuelist_name"
      class="overflow-hidden border-r border-(--p-form-field-border-color) whitespace-nowrap"
      :style="{
        paddingLeft: `${item.level}em`,
      }"
    >
      <path-icon
        :icon="item.isGroup ? (isExpanded ? mdiMenuDown : mdiMenuRight) : null"
        @click.stop="if (item.isGroup) uiState.toggleExpand(item.cue.id);"
        @pointerdown.stop
      />
      {{ item.cue.name != null ? item.cue.name : buildCueName(item.cue) }}
    </td>
    <td
      headers="cuelist_repeat"
      class="border-r border-(--p-form-field-border-color) px-0"
    >
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
    <td
      headers="cuelist_chain"
      class="px-0"
    >
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

<style lang="css" module></style>
