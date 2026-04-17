<template>
  <tr
    :class="[
      props.isDragOver ? $style['drag-over-row'] : '',
      isSelected ? $style['selected-row'] : '',
    ]"
    :draggable="uiState.mode == 'edit' ? 'true' : 'false'"
  >
    <td
      headers="cuelist_cursor"
      style="padding-left: 12px; padding-right: 0px"
    >
      <v-icon
        :icon="isPlaybackCursor ? mdiArrowRightBold : undefined"
        @click="setPlaybackCursor(props.item.cue.id)"
      />
    </td>
    <td
      headers="cuelist_status"
      style="padding-left: 6px"
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
      class="overflow-hidden text-no-wrap"
      :style="{
        paddingLeft: `${item.level}em`,
      }"
      @dblclick="openEditable($event, 'cuelist_name')"
      @blur="closeEditable($event.target, true, 'cuelist_name')"
      @keydown.enter.stop="closeEditable($event.target, true, 'cuelist_name')"
      @keydown.esc.stop="closeEditable($event.target, false, 'cuelist_name')"
    >
      <v-icon
        :icon="item.isGroup ? (isExpanded ? mdiMenuDown : mdiMenuRight) : undefined"
        @click.stop="if (item.isGroup) uiState.toggleExpand(item.cue.id);"
        @pointerdown.stop
      />
      {{ item.cue.name != null ? item.cue.name : buildCueName(item.cue) }}
    </td>
    <td
      headers="cuelist_pre_wait"
      class="text-center"
      style="padding: 0px 4px"
    >
      <div
        :class="[isPreWaitActive ? 'border-md border-primary' : '']"
        :style="preWaitProgressStyle"
        @dblclick="if (!isPreWaitActive) openEditable($event, 'cuelist_pre_wait');"
        @blur="closeEditable($event.target, true, 'cuelist_pre_wait')"
        @keydown.enter.stop="closeEditable($event.target, true, 'cuelist_pre_wait')"
        @keydown.esc.stop="closeEditable($event.target, false, 'cuelist_pre_wait')"
      >
        {{ preWaitText }}
      </div>
    </td>
    <td
      headers="cuelist_duration"
      class="text-center"
      style="padding: 0px 4px"
    >
      <div
        :class="[isActive ? 'border-md border-primary' : '']"
        :style="durationProgressStyle"
        @dblclick="if (!isActive) openEditable($event, 'cuelist_duration');"
        @blur="closeEditable($event.target, true, 'cuelist_duration')"
        @keydown.enter.stop="closeEditable($event.target, true, 'cuelist_duration')"
        @keydown.esc.stop="closeEditable($event.target, false, 'cuelist_duration')"
      >
        {{ durationText }}
      </div>
    </td>
    <td headers="cuelist_repeat">
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
    <td headers="cuelist_chain">
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
import { computed, toRaw } from 'vue';
import { useUiState } from '../../stores/uistate';
import { useShowState } from '../../stores/showstate';
import { buildCueName, calculateDuration, formatToSeconds, getCueIcon, getLockCursorToSelection, secondsToFormat } from '../../utils';
import { useApi } from '../../api';
import { useAssetResult } from '../../stores/assetResult';
import { PlaybackStatus } from '../../types/PlaybackStatus';

const api = useApi();
const uiState = useUiState();
const showState = useShowState();
const assetResult = useAssetResult();

const props = defineProps<{
  item: FlatCueEntry;
  isDragOver: boolean;
}>();

const isSelected = computed(() => uiState.selectedRows.includes(props.item.cue.id));
const isExpanded = computed(() => uiState.expandedRows.includes(props.item.cue.id));
const isPlaybackCursor = computed(() => showState.playbackCursor === props.item.cue.id);
const cueIcon = computed(() => getCueIcon(props.item.cue.params.type));
const preWaitText = computed(() => {
  return isPreWaitActive.value
    ? secondsToFormat(
        uiState.preWaitDisplayMode == 'elapsed'
          ? showState.activeCues[props.item.cue.id]!.position
          : showState.activeCues[props.item.cue.id]!.duration - showState.activeCues[props.item.cue.id]!.position,
      )
    : secondsToFormat(props.item.cue.preWait == 0.0 ? null : props.item.cue.preWait);
});
const preWaitProgressStyle = computed(() => {
  return {
    background: isPreWaitActive.value
      ? 'linear-gradient(to right, rgba(var(--v-theme-primary), 0.5) '
      + Math.floor(
        (showState.activeCues[props.item.cue.id]!.position * 100) / showState.activeCues[props.item.cue.id]!.duration,
      )
      + '%, transparent '
      + Math.floor(
        (showState.activeCues[props.item.cue.id]!.position * 100) / showState.activeCues[props.item.cue.id]!.duration,
      )
      + '%) no-repeat'
      : '',
  };
});
const durationText = computed(() => {
  return isActive.value
    ? secondsToFormat(
        uiState.durationDisplayMode == 'elapsed'
          ? showState.activeCues[props.item.cue.id]!.position
          : showState.activeCues[props.item.cue.id]!.duration - showState.activeCues[props.item.cue.id]!.position,
      )
    : secondsToFormat(calculateDuration(props.item.cue.params, assetResult.getMetadata(props.item.cue.id)?.duration));
});
const durationProgressStyle = computed(() => {
  return {
    background: isActive.value
      ? 'linear-gradient(to right, rgba(var(--v-theme-primary), 0.5) '
      + Math.floor(
        (showState.activeCues[props.item.cue.id]!.position * 100) / showState.activeCues[props.item.cue.id]!.duration,
      )
      + '%, transparent '
      + Math.floor(
        (showState.activeCues[props.item.cue.id]!.position * 100) / showState.activeCues[props.item.cue.id]!.duration,
      )
      + '%) no-repeat'
      : '',
  };
});

const setPlaybackCursor = (cueId: string) => {
  if (!getLockCursorToSelection()) {
    api.setPlaybackCursor(cueId);
  }
};

const openEditable = (e: MouseEvent, editType: string) => {
  if (uiState.mode != 'edit') {
    return;
  }
  if (e.target == null || !(e.target instanceof HTMLElement) || e.target.contentEditable === 'true') {
    return;
  }
  if (props.item == null) return;
  if (editType == 'cuelist_duration') {
    const cueType = props.item.cue.params.type;
    if (cueType != 'wait' && cueType != 'fade') {
      return;
    }
  }
  if (editType == 'cuelist_post_wait') {
    if (props.item.isChainOverrided) {
      return;
    }
    if (props.item.chain.type != 'afterStart') {
      return;
    }
  }
  e.target.contentEditable = 'true';
  e.target.classList.add('inEdit');
  e.target.dataset.prevText = e.target.innerText;
  var range = document.createRange();
  range.selectNodeContents(e.target);
  var sel = window.getSelection();
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
        if (newText == '') {
          newCue.name = null;
        } else {
          newCue.name = newText;
        }
        break;
      }
      case 'cuelist_pre_wait': {
        let newPreWait = formatToSeconds(target.innerText, false);
        newCue.preWait = newPreWait;
        break;
      }
      case 'cuelist_duration': {
        if (newCue.params.type == 'wait') {
          let newDuration = formatToSeconds(target.innerText, false);
          newCue.params.duration = newDuration;
        } else if (newCue.params.type == 'fade') {
          let newDuration = formatToSeconds(target.innerText, false);
          newCue.params.fadeParam.duration = newDuration;
        }
        break;
      }
    }
    api.updateCue(newCue);
  } else {
    if (target.dataset.prevText != undefined) {
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
    props.item.cue.id in showState.activeCues
    && (['preWaiting', 'preWaitPaused'] as PlaybackStatus[]).includes(showState.activeCues[props.item.cue.id]!.status)
  );
});

const isActive = computed((): boolean => {
  return (
    props.item.cue.id in showState.activeCues
    && (['playing', 'paused', 'stopping', 'completed'] as PlaybackStatus[]).includes(
      showState.activeCues[props.item.cue.id]!.status,
    )
  );
});
</script>

<style lang="css" module>
  .drag-over-row > td {
    border-top: 2px solid rgb(var(--v-theme-primary)) !important;
  }
  .selected-row > td {
    background-color: rgb(var(--v-theme-primary), 0.2);
  }
</style>
