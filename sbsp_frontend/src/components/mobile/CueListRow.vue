<template>
  <tr
    :style="backgroundStyle"
  >
    <td
      headers="cuelist_cursor"
      class="px-1"
    >
      <v-icon
        :icon="isPlaybackCursor ? mdiArrowRightBold : undefined"
      />
    </td>
    <td
      headers="cuelist_status"
      class="px-1"
    >
      <v-icon
        v-show="isStatusIn(['playing', 'preWaiting'])"
        :icon="mdiPlay"
        color="success"
      />
      <v-icon
        v-show="isStatusIn(['paused', 'preWaitPaused'])"
        :icon="mdiPause"
        color="warning"
      />
      <v-icon
        v-show="status == 'loaded'"
        :icon="mdiUpload"
        color="warning"
      />
      <v-progress-circular
        v-show="status == 'stopping'"
        indeterminate="disable-shrink"
        size="16"
        color="warning"
      />
    </td>
    <td
      headers="cuelist_type"
      class="text-center"
      style="padding: 0px"
    >
      <v-icon :icon="cueIcon" />
    </td>
    <td
      headers="cuelist_number"
      class="text-center border-e border-s px-1"
    >
      {{ item.cue.number }}
    </td>
    <td
      headers="cuelist_name"
      class="border-e overflow-hidden text-no-wrap"
      :style="{
        paddingLeft: `${item.level}em`,
      }"
    >
      <v-icon
        :icon="item.isGroup ? (isExpanded ? mdiMenuDown : mdiMenuRight) : undefined"
        @click.stop="if (item.isGroup) uiState.toggleExpand(item.cue.id);"
        @pointerdown.stop
      />
      {{ item.cue.name != null ? item.cue.name : buildCueName(item.cue) }}
    </td>
    <td headers="cuelist_repeat" class="px-0 border-e">
      <v-icon
        v-show="
          (item.cue.params.type == 'audio' && item.cue.params.repeat) ||
            (item.cue.params.type == 'group' &&
              item.cue.params.mode.type == 'playlist' &&
              item.cue.params.mode.repeat)
        "
        :icon="mdiRepeat"
      />
    </td>
    <td headers="cuelist_chain" class="px-0">
      <v-icon
        v-show="item.chain.type == 'afterComplete'"
        :icon="mdiArrowCollapseDown"
      />
      <v-icon
        v-show="item.chain.type == 'afterStart'"
        :icon="mdiArrowExpandDown"
      />
    </td>
  </tr>
</template>

<script setup lang="ts">
import { mdiArrowCollapseDown, mdiArrowExpandDown, mdiArrowRightBold, mdiMenuDown, mdiMenuRight, mdiPause, mdiPlay, mdiRepeat, mdiUpload } from '@mdi/js';
import { FlatCueEntry } from '../../stores/showmodel';
import { computed } from 'vue';
import { useUiState } from '../../stores/uistate';
import { useShowState } from '../../stores/showstate';
import { buildCueName, getCueIcon } from '../../utils';
import { PlaybackStatus } from '../../types/PlaybackStatus';

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

const backgroundStyle = computed(() => {
  return {
    background: showState.activeCues[props.item.cue.id] != null
      ? (showState.activeCues[props.item.cue.id]!.status.startsWith('pre')
          ? 'linear-gradient(to right, rgba(var(--v-theme-warning), 0.5) '
          : 'linear-gradient(to right, rgba(var(--v-theme-primary), 0.5) ')
        + (showState.activeCues[props.item.cue.id]!.position * 100) / showState.activeCues[props.item.cue.id]!.duration
        + '%, transparent '
        + (showState.activeCues[props.item.cue.id]!.position * 100) / showState.activeCues[props.item.cue.id]!.duration
        + '%) no-repeat'
      : '',
  };
});

const isStatusIn = (statusList: PlaybackStatus[]): boolean => {
  return status.value != null ? statusList.includes(status.value) : false;
};
</script>

<style lang="css" module>
</style>
