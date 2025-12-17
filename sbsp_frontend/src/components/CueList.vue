<template>
  <v-table fixed-header density="compact" class="flex-grow-1" height="100%" @mousedown="resetSelection">
    <thead>
      <tr>
        <th id="cuelist_cursor" width="53px"></th>
        <th id="cuelist_type" width="53px"></th>
        <th id="cuelist_number" class="text-center" width="60px">{{ t('main.number') }}</th>
        <th id="cuelist_name">{{ t('main.name') }}</th>
        <th id="cuelist_pre_wait" class="text-center" width="100px">{{ t('main.preWait') }}</th>
        <th id="cuelist_duration" class="text-center" width="100px">{{ t('main.duration') }}</th>
        <th id="cuelist_post_wait" class="text-center" width="100px">{{ t('main.postWait') }}</th>
        <th id="cuelist_repeat" width="53px"><v-icon :icon="mdiRepeat"></v-icon></th>
        <th id="cuelist_sequence" width="53px"><v-icon :icon="mdiChevronDoubleDown" /></th>
      </tr>
    </thead>
    <tbody>
      <tr
        v-for="(item, i) in showModel.flatCueList"
        v-show="!item.isHidden"
        ref="cuelistItem"
        :key="item.cue.id"
        :class="[
          dragOverIndex == i ? $style['drag-over-row'] : '',
          uiState.selectedRows.includes(item.cue.id) ? $style['selected-row'] : '',
        ]"
        style="scroll-margin-top: 40px"
        draggable="true"
        @dragstart="dragStart($event, i)"
        @dragover="dragOver($event, i)"
        @dragend="dragEnd"
        @drop="drop($event, i)"
        @mousedown.stop="click($event, i)"
      >
        <td headers="cuelist_cursor" width="53px">
          <v-icon
            :icon="showState.playbackCursor == item.cue.id ? mdiArrowRightBold : undefined"
            @click="setPlaybackCursor(item.cue.id)"
          ></v-icon>
        </td>
        <td headers="cuelist_type" width="53px">
          <v-icon :icon="getCueIcon(item.cue.params.type)" />
        </td>
        <td
          headers="cuelist_number"
          class="text-center"
          width="50px"
          @dblclick="openEditable($event, i, 'cuelist_number')"
          @blur="closeEditable($event.target, true, i, 'cuelist_number')"
          @keydown.enter.stop="closeEditable($event.target, true, i, 'cuelist_number')"
          @keydown.esc.stop="closeEditable($event.target, false, i, 'cuelist_number')"
        >
          {{ item.cue.number }}
        </td>
        <td
          headers="cuelist_name"
          width="auto"
          :style="{
            paddingLeft: `${item.level}em`,
          }"
          @dblclick="openEditable($event, i, 'cuelist_name')"
          @blur="closeEditable($event.target, true, i, 'cuelist_name')"
          @keydown.enter.stop="closeEditable($event.target, true, i, 'cuelist_name')"
          @keydown.esc.stop="closeEditable($event.target, false, i, 'cuelist_name')"
        >
          <v-icon
            :icon="item.isGroup ? (uiState.expandedRows.includes(item.cue.id) ? mdiMenuDown : mdiMenuRight) : undefined"
            @click.stop="if (item.isGroup) uiState.toggleExpand(item.cue.id);"
            @mousedown.stop
          ></v-icon>
          {{ item.cue.name != null ? item.cue.name : buildCueName(item.cue) }}
        </td>
        <td headers="cuelist_pre_wait" class="text-center pa-1" width="100px">
          <div
            :class="[isPreWaitActive(item.cue.id) ? 'border-md border-primary' : '']"
            :style="{
              background: isPreWaitActive(item.cue.id)
                ? 'linear-gradient(to right, rgba(var(--v-theme-primary), 0.5) ' +
                  Math.floor(
                    (showState.activeCues[item.cue.id]!.position * 100) / showState.activeCues[item.cue.id]!.duration,
                  ) +
                  '%, transparent ' +
                  Math.floor(
                    (showState.activeCues[item.cue.id]!.position * 100) / showState.activeCues[item.cue.id]!.duration,
                  ) +
                  '%) no-repeat'
                : '',
            }"
            @dblclick="if (!isPreWaitActive(item.cue.id)) openEditable($event, i, 'cuelist_pre_wait');"
            @blur="closeEditable($event.target, true, i, 'cuelist_pre_wait')"
            @keydown.enter.stop="closeEditable($event.target, true, i, 'cuelist_pre_wait')"
            @keydown.esc.stop="closeEditable($event.target, false, i, 'cuelist_pre_wait')"
          >
            {{
              isPreWaitActive(item.cue.id)
                ? secondsToFormat(showState.activeCues[item.cue.id]!.position)
                : secondsToFormat(item.cue.preWait == 0.0 ? null : item.cue.preWait)
            }}
          </div>
        </td>
        <td headers="cuelist_duration" class="text-center pa-1" width="100px">
          <div
            :class="[isActive(item.cue.id) ? 'border-md border-primary' : '']"
            :style="{
              background: isActive(item.cue.id)
                ? 'linear-gradient(to right, rgba(var(--v-theme-primary), 0.5) ' +
                  Math.floor(
                    (showState.activeCues[item.cue.id]!.position * 100) / showState.activeCues[item.cue.id]!.duration,
                  ) +
                  '%, transparent ' +
                  Math.floor(
                    (showState.activeCues[item.cue.id]!.position * 100) / showState.activeCues[item.cue.id]!.duration,
                  ) +
                  '%) no-repeat'
                : '',
            }"
            @dblclick="if (!isActive(item.cue.id)) openEditable($event, i, 'cuelist_duration');"
            @blur="closeEditable($event.target, true, i, 'cuelist_duration')"
            @keydown.enter.stop="closeEditable($event.target, true, i, 'cuelist_duration')"
            @keydown.esc.stop="closeEditable($event.target, false, i, 'cuelist_duration')"
          >
            {{
              isActive(item.cue.id)
                ? secondsToFormat(showState.activeCues[item.cue.id]!.position)
                : secondsToFormat(calculateDuration(item.cue.params, assetResult.get(item.cue.id)?.duration))
            }}
          </div>
        </td>
        <td headers="cuelist_post_wait" class="text-center pa-1" width="100px">
          <div
            :class="
              isActive(item.cue.id) &&
              item.cue.sequence.type == 'autoContinue' &&
              showState.activeCues[item.cue.id]!.position < item.cue.sequence.postWait
                ? 'border-md border-primary'
                : ''
            "
            :style="{
              background:
                item.cue.sequence.type != 'doNotContinue' &&
                isActive(item.cue.id) &&
                showState.activeCues[item.cue.id]!.position <
                  (item.cue.sequence.type == 'autoContinue'
                    ? item.cue.sequence.postWait
                    : showState.activeCues[item.cue.id]!.duration)
                  ? 'linear-gradient(to right, rgba(var(--v-theme-primary), 0.5) ' +
                    Math.floor(
                      (showState.activeCues[item.cue.id]!.position * 100) /
                        (item.cue.sequence.type == 'autoContinue'
                          ? item.cue.sequence.postWait
                          : showState.activeCues[item.cue.id]!.duration),
                    ) +
                    '%, transparent ' +
                    Math.floor(
                      (showState.activeCues[item.cue.id]!.position * 100) /
                        (item.cue.sequence.type == 'autoContinue'
                          ? item.cue.sequence.postWait
                          : showState.activeCues[item.cue.id]!.duration),
                    ) +
                    '%) no-repeat'
                  : '',
            }"
            @dblclick="if (!isActive(item.cue.id)) openEditable($event, i, 'cuelist_post_wait');"
            @blur="closeEditable($event.target, true, i, 'cuelist_post_wait')"
            @keydown.enter.stop="closeEditable($event.target, true, i, 'cuelist_post_wait')"
            @keydown.esc.stop="closeEditable($event.target, false, i, 'cuelist_post_wait')"
          >
            {{
              item.cue.sequence.type == 'doNotContinue'
                ? '--:--.--'
                : isActive(item.cue.id) &&
                    showState.activeCues[item.cue.id]!.position <
                      (item.cue.sequence.type == 'autoContinue'
                        ? item.cue.sequence.postWait
                        : showState.activeCues[item.cue.id]!.duration)
                  ? secondsToFormat(showState.activeCues[item.cue.id]!.position)
                  : item.cue.sequence.type == 'autoContinue'
                    ? secondsToFormat(item.cue.sequence.postWait)
                    : secondsToFormat(calculateDuration(item.cue.params, assetResult.get(item.cue.id)?.duration))
            }}
          </div>
        </td>
        <td headers="cuelist_repeat" width="53px">
          <v-icon v-if="item.cue.params.type == 'audio' && item.cue.params.repeat" :icon="mdiRepeat" />
        </td>
        <td headers="cuelist_sequence" width="53px">
          <v-icon v-if="item.cue.sequence.type == 'autoFollow'" :icon="mdiArrowExpandDown" />
          <v-icon v-if="item.cue.sequence.type == 'autoContinue'" :icon="mdiArrowDown" />
        </td>
      </tr>
      <tr
        :class="dragOverIndex == showModel.flatCueList.length ? $style['drag-over-row'] : ''"
        @dragover="dragOver($event, showModel.flatCueList.length)"
        @drop="drop($event, showModel.flatCueList.length)"
      >
        <td headers="cuelist_cursor"></td>
        <td headers="cuelist_type"></td>
        <td headers="cuelist_number"></td>
        <td headers="cuelist_name"></td>
        <td headers="cuelist_pre_wait"></td>
        <td headers="cuelist_duration"></td>
        <td headers="cuelist_post_wait"></td>
        <td headers="cuelist_repeat"></td>
        <td headers="cuelist_sequence"></td>
      </tr>
    </tbody>
  </v-table>
</template>

<script setup lang="ts">
import { ref, toRaw, useTemplateRef } from 'vue';
import { useShowModel } from '../stores/showmodel';
import {
  mdiArrowDown,
  mdiArrowExpandDown,
  mdiArrowRightBold,
  mdiChartBellCurveCumulative,
  mdiChevronDoubleDown,
  mdiGroup,
  mdiMenuDown,
  mdiMenuRight,
  mdiPauseCircleOutline,
  mdiPlayCircleOutline,
  mdiRepeat,
  mdiStopCircleOutline,
  mdiTimerSandEmpty,
  mdiUploadCircleOutline,
  mdiVolumeHigh,
} from '@mdi/js';
import { useUiState } from '../stores/uistate';
import { useShowState } from '../stores/showstate';
import { invoke } from '@tauri-apps/api/core';
import { buildCueName, calculateDuration, formatToSeconds, getLockCursorToSelection, secondsToFormat } from '../utils';
import type { PlaybackStatus } from '../types/PlaybackStatus';
import { useHotkey } from 'vuetify';
import { useAssetResult } from '../stores/assetResult';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();

const showModel = useShowModel();
const showState = useShowState();
const uiState = useUiState();
const assetResult = useAssetResult();

const cueListItemRefs = useTemplateRef('cuelistItem');

const scrollIntoIndex = (inddex: number) => {
  if (cueListItemRefs.value != null) {
    cueListItemRefs.value[inddex].scrollIntoView({ block: 'nearest' });
  }
};

const onArrowUp = (e: KeyboardEvent) => {
  if (uiState.selected != null) {
    let cursorIndex = showModel.flatCueList.findIndex((item) => item.cue.id == uiState.selected) - 1;
    if (cursorIndex < 0) return;
    while (showModel.flatCueList[cursorIndex].isHidden) {
      cursorIndex--;
      if (cursorIndex < 0) {
        return;
      }
    }
    if (e.shiftKey) {
      uiState.addSelected(showModel.flatCueList[cursorIndex].cue.id);
    } else {
      uiState.setSelected(showModel.flatCueList[cursorIndex].cue.id);
    }
    scrollIntoIndex(cursorIndex);
  } else if (showModel.flatCueList.length > 0) {
    uiState.setSelected(showModel.flatCueList[0].cue.id);
    scrollIntoIndex(0);
  }
};

useHotkey('arrowup', onArrowUp);
useHotkey('shift+arrowup', onArrowUp);

const onArrowDown = (e: KeyboardEvent) => {
  if (uiState.selected != null) {
    let cursorIndex = showModel.flatCueList.findIndex((item) => item.cue.id == uiState.selected) + 1;
    if (cursorIndex >= showModel.flatCueList.length) return;
    while (showModel.flatCueList[cursorIndex].isHidden) {
      if (cursorIndex >= showModel.flatCueList.length) {
        return;
      }
      cursorIndex++;
    }
    if (e.shiftKey) {
      uiState.addSelected(showModel.flatCueList[cursorIndex].cue.id);
    } else {
      uiState.setSelected(showModel.flatCueList[cursorIndex].cue.id);
    }
    scrollIntoIndex(cursorIndex);
  } else if (showModel.flatCueList.length > 0) {
    uiState.setSelected(showModel.flatCueList[showModel.flatCueList.length - 1].cue.id);
    scrollIntoIndex(showModel.flatCueList.length - 1);
  }
};

useHotkey('arrowdown', onArrowDown);
useHotkey('shift+arrowdown', onArrowDown);

useHotkey('cmd+a', () => {
  uiState.selectedRows = showModel.flatCueList.filter((item) => !item.isHidden).map((item) => item.cue.id);
});

useHotkey('cmd+backspace', () => {
  for (const row of uiState.selectedRows) {
    invoke('remove_cue', { cueId: row }).catch((e) => console.error(e));
  }
});

const dragOverIndex = ref();

const dragStart = (event: DragEvent, index: number) => {
  if (event.dataTransfer) {
    event.dataTransfer.effectAllowed = 'move';
    event.dataTransfer.dropEffect = 'move';
    event.dataTransfer.setData('text/plain', index.toString());
  }
};

const dragOver = (event: DragEvent, index: number) => {
  event.preventDefault();
  dragOverIndex.value = index;
};

const dragEnd = () => {
  dragOverIndex.value = null;
};

const drop = (event: DragEvent, index: number) => {
  event.preventDefault();
  if (event.dataTransfer) {
    const fromIndex = Number(event.dataTransfer.getData('text/plain'));
    if (fromIndex === index) {
      return;
    }
    const cueId = showModel.flatCueList[fromIndex].cue.id;
    const targetId = showModel.flatCueList[index].cue.id;
    invoke('move_cue', { cueId: cueId, targetId: targetId }).catch((e) => {
      console.log('Failed to move cue. ' + e);
    });
    // showModel.moveCue(cue_id, newIndex);
  }
};

const click = (event: MouseEvent, index: number) => {
  const clickedId = showModel.flatCueList[index].cue.id;
  if (event.shiftKey) {
    if (uiState.selected != null) {
      uiState.selectedRows = [];
      const prevIndex = showModel.flatCueList.findIndex((item) => item.cue.id === uiState.selected);
      if (index >= prevIndex) {
        for (let i = prevIndex; i <= index; i++) {
          uiState.selectedRows.push(showModel.flatCueList[i].cue.id);
        }
      } else {
        for (let i = index; i <= prevIndex; i++) {
          uiState.selectedRows.push(showModel.flatCueList[i].cue.id);
        }
      }
    } else {
      uiState.selectedRows = [clickedId];
    }
    uiState.selected = clickedId;
  } else if (event.ctrlKey) {
    if (uiState.selected != null) {
      if (uiState.selectedRows.includes(clickedId)) {
        uiState.selectedRows.splice(
          uiState.selectedRows.findIndex((row) => row === clickedId),
          1,
        );
        if (uiState.selectedRows.length === 0) {
          uiState.selected = null;
        } else if (index === showModel.flatCueList.findIndex((item) => item.cue.id == showState.playbackCursor)) {
          const findIdx = (x: string): number => showModel.flatCueList.findIndex((item) => item.cue.id === x);
          uiState.selected = uiState.selectedRows.reduce((a, b) => {
            return findIdx(a) > findIdx(b) ? a : b;
          });
        }
      } else {
        uiState.selectedRows.push(clickedId);
        uiState.selected = clickedId;
      }
    } else {
      uiState.selectedRows = [clickedId];
      uiState.selected = clickedId;
    }
  } else {
    uiState.selectedRows = [clickedId];
    uiState.selected = clickedId;
  }
  if (getLockCursorToSelection()) {
    invoke('set_playback_cursor', {
      cueId: uiState.selected !== null ? uiState.selected : null,
    }).catch((e) => {
      console.error('Failed to set cursor. ' + e);
    });
  }
};

const resetSelection = () => {
  uiState.selectedRows = [];
  uiState.selected = null;
  if (getLockCursorToSelection()) {
    invoke('set_playback_cursor', {
      cueId: uiState.selected !== null ? uiState.selected : null,
    }).catch((e) => {
      console.error('Failed to set cursor. ' + e);
    });
  }
};

const getCueIcon = (type: string): string | undefined => {
  switch (type) {
    case 'audio':
      return mdiVolumeHigh;
    case 'wait':
      return mdiTimerSandEmpty;
    case 'fade':
      return mdiChartBellCurveCumulative;
    case 'start':
      return mdiPlayCircleOutline;
    case 'stop':
      return mdiStopCircleOutline;
    case 'pause':
      return mdiPauseCircleOutline;
    case 'load':
      return mdiUploadCircleOutline;
    case 'group':
      return mdiGroup;
  }
};

const openEditable = (e: MouseEvent, rowIndex: number, editType: string) => {
  if (e.target == null || !(e.target instanceof HTMLElement) || e.target.contentEditable === 'true') {
    return;
  }
  if (editType == 'cuelist_duration') {
    const cueType = showModel.flatCueList[rowIndex].cue.params.type;
    if (cueType != 'wait' && cueType != 'fade') {
      return;
    }
  }
  if (editType == 'cuelist_post_wait') {
    if (showModel.flatCueList[rowIndex].cue.sequence.type != 'autoContinue') {
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

const closeEditable = (target: EventTarget | null, needSave: boolean, rowIndex: number, editType: string) => {
  if (target == null || !(target instanceof HTMLElement) || target.contentEditable === 'false') {
    return;
  }
  target.contentEditable = 'false';
  target.classList.remove('inEdit');
  if (needSave) {
    const newCue = structuredClone(toRaw(showModel.flatCueList[rowIndex].cue));
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
      case 'cuelist_post_wait': {
        if (newCue.sequence.type == 'autoContinue') {
          let newPostWait = formatToSeconds(target.innerText);
          newCue.sequence.postWait = newPostWait;
        }
        break;
      }
    }
    invoke('update_cue', { cue: newCue }).catch((e) => console.log(e.toString()));
  } else {
    if (target.dataset.prevText != undefined) {
      target.innerText = target.dataset.prevText;
    }
  }
  delete target.dataset.prevText;
};

const isPreWaitActive = (cue_id: string): boolean => {
  return (
    cue_id in showState.activeCues &&
    (['PreWaiting', 'PreWaitPaused'] as PlaybackStatus[]).includes(showState.activeCues[cue_id]!.status)
  );
};

const isActive = (cue_id: string): boolean => {
  return (
    cue_id in showState.activeCues &&
    (['Playing', 'Paused', 'Stopping', 'Completed'] as PlaybackStatus[]).includes(showState.activeCues[cue_id]!.status)
  );
};

const setPlaybackCursor = (cueId: string) => {
  if (!getLockCursorToSelection()) {
    invoke('set_playback_cursor', { cueId: cueId }).catch((e) => console.error(e));
  }
};
</script>

<style lang="css" module>
.drag-over-row > td {
  border-top: 2px solid rgb(var(--v-theme-primary)) !important;
}
.selected-row > td {
  background-color: rgb(var(--v-theme-primary), 0.2);
  color: rgb(var(--v-theme-on-background));
}
</style>
