<template>
  <v-table fixed-header density="compact" class="flex-grow-1" :class="$style['cuelist']" height="100%">
    <thead>
      <tr>
        <th id="cuelist_cursor" width="32px"></th>
        <th id="cuelist_status" width="32px"></th>
        <th id="cuelist_type" width="24px"></th>
        <th id="cuelist_number" class="text-center border-s" width="54px" style="padding: 0">#</th>
        <th id="cuelist_name" class="border overflow-hidden text-no-wrap" style="padding-left: 24px">
          {{ t('main.name') }}
        </th>
        <th id="cuelist_pre_wait" class="text-center" width="104px" style="padding: 0px 8px">
          <div class="d-flex flex-row justify-center ga-1">
            {{ t('main.preWait') }}
            <v-icon
              class="mt-auto mb-auto"
              :icon="uiState.preWaitDisplayMode == 'elapsed' ? mdiAlphaEBoxOutline : mdiAlphaRBoxOutline"
              @click.stop="uiState.togglePreWaitDisplayMode"
            ></v-icon>
          </div>
        </th>
        <th id="cuelist_duration" class="text-center" width="104px" style="padding: 0px 8px">
          <div class="d-flex flex-row justify-center ga-1">
            {{ t('main.duration') }}
            <v-icon
              class="mt-auto mb-auto"
              :icon="uiState.durationDisplayMode == 'elapsed' ? mdiAlphaEBoxOutline : mdiAlphaRBoxOutline"
              @click.stop="uiState.toggleDurationDisplayMode"
            ></v-icon>
          </div>
        </th>
        <th id="cuelist_post_wait" class="text-center" width="104px" style="padding: 0px 8px">
          <div class="d-flex flex-row justify-center ga-1">
            {{ t('main.postWait') }}
            <v-icon
              class="mt-auto mb-auto"
              :icon="uiState.postWaitDisplayMode == 'elapsed' ? mdiAlphaEBoxOutline : mdiAlphaRBoxOutline"
              @click.stop="uiState.togglePostWaitDisplayMode"
            ></v-icon>
          </div>
        </th>
        <th id="cuelist_repeat" width="32px"><v-icon :icon="mdiRepeat"></v-icon></th>
        <th id="cuelist_sequence" width="54px"><v-icon :icon="mdiChevronDoubleDown" /></th>
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
        :draggable="uiState.mode == 'edit' ? 'true' : 'false'"
        @dragstart="dragStart($event, i)"
        @dragover="dragOver($event, i)"
        @dragend="dragEnd"
        @drop="drop($event, i)"
        @mousedown.stop="click($event, i)"
      >
        <td headers="cuelist_cursor" style="padding-left: 12px; padding-right: 0px">
          <v-icon
            :icon="showState.playbackCursor == item.cue.id ? mdiArrowRightBold : undefined"
            @click="setPlaybackCursor(item.cue.id)"
          ></v-icon>
        </td>
        <td headers="cuelist_status" style="padding-left: 6px">
          <v-icon
            v-show="['Playing', 'PreWaiting'].includes(getStatus(item.cue.id))"
            :icon="mdiPlay"
            color="success"
          ></v-icon>
          <v-icon
            v-show="['Paused', 'PreWaitPaused'].includes(getStatus(item.cue.id))"
            :icon="mdiPause"
            color="warning"
          ></v-icon>
          <v-icon v-show="getStatus(item.cue.id) == 'Loaded'" :icon="mdiUpload" color="warning"></v-icon>
          <v-progress-circular
            v-show="getStatus(item.cue.id) == 'Stopping'"
            indeterminate="disable-shrink"
            size="16"
            color="warning"
          ></v-progress-circular>
        </td>
        <td headers="cuelist_type" class="text-center" style="padding: 0px">
          <v-icon :icon="getCueIcon(item.cue.params.type)" />
        </td>
        <td
          headers="cuelist_number"
          class="text-center"
          @dblclick="openEditable($event, i, 'cuelist_number')"
          @blur="closeEditable($event.target, true, i, 'cuelist_number')"
          @keydown.enter.stop="closeEditable($event.target, true, i, 'cuelist_number')"
          @keydown.esc.stop="closeEditable($event.target, false, i, 'cuelist_number')"
        >
          {{ item.cue.number }}
        </td>
        <td
          headers="cuelist_name"
          class="overflow-hidden text-no-wrap"
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
        <td headers="cuelist_pre_wait" class="text-center" style="padding: 0px 4px">
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
                ? secondsToFormat(
                    uiState.preWaitDisplayMode == 'elapsed'
                      ? showState.activeCues[item.cue.id]!.position
                      : showState.activeCues[item.cue.id]!.duration - showState.activeCues[item.cue.id]!.position,
                  )
                : secondsToFormat(item.cue.preWait == 0.0 ? null : item.cue.preWait)
            }}
          </div>
        </td>
        <td headers="cuelist_duration" class="text-center" style="padding: 0px 4px">
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
                ? secondsToFormat(
                    uiState.durationDisplayMode == 'elapsed'
                      ? showState.activeCues[item.cue.id]!.position
                      : showState.activeCues[item.cue.id]!.duration - showState.activeCues[item.cue.id]!.position,
                  )
                : secondsToFormat(calculateDuration(item.cue.params, assetResult.get(item.cue.id)?.duration))
            }}
          </div>
        </td>
        <td headers="cuelist_post_wait" class="text-center" style="padding: 0px 4px">
          <div
            :class="
              isActive(item.cue.id) &&
              (item.sequence.type == 'autoFollow' ||
                (item.sequence.type == 'autoContinue' &&
                  showState.activeCues[item.cue.id]!.position < item.sequence.postWait))
                ? 'border-md border-primary'
                : ''
            "
            :style="{
              background:
                item.sequence.type != 'doNotContinue' &&
                isActive(item.cue.id) &&
                showState.activeCues[item.cue.id]!.position <
                  (item.sequence.type == 'autoContinue'
                    ? item.sequence.postWait
                    : showState.activeCues[item.cue.id]!.duration)
                  ? 'linear-gradient(to right, rgba(var(--v-theme-primary), 0.5) ' +
                    Math.floor(
                      (showState.activeCues[item.cue.id]!.position * 100) /
                        (item.sequence.type == 'autoContinue'
                          ? item.sequence.postWait
                          : showState.activeCues[item.cue.id]!.duration),
                    ) +
                    '%, transparent ' +
                    Math.floor(
                      (showState.activeCues[item.cue.id]!.position * 100) /
                        (item.sequence.type == 'autoContinue'
                          ? item.sequence.postWait
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
              item.sequence.type == 'doNotContinue'
                ? '--:--.--'
                : isActive(item.cue.id) &&
                    showState.activeCues[item.cue.id]!.position <
                      (item.sequence.type == 'autoContinue'
                        ? item.sequence.postWait
                        : showState.activeCues[item.cue.id]!.duration)
                  ? secondsToFormat(
                      uiState.postWaitDisplayMode == 'elapsed'
                        ? showState.activeCues[item.cue.id]!.position
                        : showState.activeCues[item.cue.id]!.duration - showState.activeCues[item.cue.id]!.position,
                    )
                  : item.sequence.type == 'autoContinue'
                    ? secondsToFormat(item.sequence.postWait)
                    : secondsToFormat(calculateDuration(item.cue.params, assetResult.get(item.cue.id)?.duration))
            }}
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
        <td headers="cuelist_sequence">
          <v-icon v-show="item.sequence.type == 'autoFollow'" :icon="mdiArrowExpandDown" />
          <v-icon v-show="item.sequence.type == 'autoContinue'" :icon="mdiArrowDown" />
        </td>
      </tr>
      <tr
        :class="dragOverIndex == showModel.flatCueList.length ? $style['drag-over-row'] : ''"
        @dragover="dragOver($event, showModel.flatCueList.length)"
        @drop="drop($event, showModel.flatCueList.length)"
      >
        <td headers="cuelist_cursor"></td>
        <td headers="cuelist_status"></td>
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
    mdiAlphaEBoxOutline,
    mdiAlphaRBoxOutline,
    mdiArrowDown,
    mdiArrowExpandDown,
    mdiArrowRightBold,
    mdiChartBellCurveCumulative,
    mdiChevronDoubleDown,
    mdiGroup,
    mdiMenuDown,
    mdiMenuRight,
    mdiPause,
    mdiPauseCircleOutline,
    mdiPlay,
    mdiPlayCircleOutline,
    mdiRepeat,
    mdiStopCircleOutline,
    mdiTimerSandEmpty,
    mdiUpload,
    mdiUploadCircleOutline,
    mdiVolumeHigh,
  } from '@mdi/js';
  import { useUiState } from '../stores/uistate';
  import { useShowState } from '../stores/showstate';
  import {
    buildCueName,
    calculateDuration,
    formatToSeconds,
    getLockCursorToSelection,
    secondsToFormat,
  } from '../utils';
  import type { PlaybackStatus } from '../types/PlaybackStatus';
  import { useHotkey } from 'vuetify';
  import { useAssetResult } from '../stores/assetResult';
  import { useI18n } from 'vue-i18n';
  import { throttle } from 'vuetify/lib/util/throttle.mjs';
  import { useApi } from '../api';

  const { t } = useI18n();
  const api = useApi();

  const showModel = useShowModel();
  const showState = useShowState();
  const uiState = useUiState();
  const assetResult = useAssetResult();

  const cueListItemRefs = useTemplateRef('cuelistItem');

  const scrollIntoIndex = (index: number) => {
    if (cueListItemRefs.value != null) {
      cueListItemRefs.value[index]?.scrollIntoView({ block: 'nearest' });
    }
  };

  const onArrowUp = throttle((e: KeyboardEvent) => {
    if (uiState.selected != null) {
      let cursorIndex = showModel.flatCueList.findIndex((item) => item.cue.id == uiState.selected) - 1;
      let cursorCueRef = showModel.flatCueList[cursorIndex];
      if (cursorCueRef == null) return;

      while (cursorCueRef.isHidden) {
        cursorIndex--;
        cursorCueRef = showModel.flatCueList[cursorIndex];
        if (cursorCueRef == null) {
          return;
        }
      }
      if (e.shiftKey) {
        uiState.addSelected(cursorCueRef.cue.id);
      } else {
        uiState.setSelected(cursorCueRef.cue.id);
      }
      scrollIntoIndex(cursorIndex);
    } else {
      const firstCueId = showModel.flatCueList[0]?.cue.id;
      if (firstCueId != null) {
        uiState.setSelected(firstCueId);
        scrollIntoIndex(0);
      }
    }
  }, 100);

  useHotkey('arrowup', onArrowUp);
  useHotkey('shift+arrowup', onArrowUp);

  const onArrowDown = throttle((e: KeyboardEvent) => {
    if (uiState.selected != null) {
      let cursorIndex = showModel.flatCueList.findIndex((item) => item.cue.id == uiState.selected) + 1;
      let cursorCueRef = showModel.flatCueList[cursorIndex];
      if (cursorCueRef == null) return;

      while (cursorCueRef.isHidden) {
        cursorIndex++;
        cursorCueRef = showModel.flatCueList[cursorIndex];
        if (cursorCueRef == null) {
          return;
        }
      }
      if (e.shiftKey) {
        uiState.addSelected(cursorCueRef.cue.id);
      } else {
        uiState.setSelected(cursorCueRef.cue.id);
      }
      scrollIntoIndex(cursorIndex);
    } else {
      const lastCueId = showModel.flatCueList[showModel.flatCueList.length - 1]?.cue.id;
      if (lastCueId != null) {
        uiState.setSelected(lastCueId);
        scrollIntoIndex(showModel.flatCueList.length - 1);
      }
    }
  }, 100);

  useHotkey('arrowdown', onArrowDown);
  useHotkey('shift+arrowdown', onArrowDown);

  useHotkey('cmd+a', () => {
    uiState.selectedRows = showModel.flatCueList.filter((item) => !item.isHidden).map((item) => item.cue.id);
  });

  useHotkey('cmd+backspace', () => {
    if (uiState.mode == 'edit') {
      for (const row of uiState.selectedRows) {
        api.removeCue(row);
      }
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
      const srcCueId = showModel.flatCueList[fromIndex]?.cue.id;
      if (srcCueId == undefined) return;
      if (index < showModel.flatCueList.length) {
        const targetId = showModel.flatCueList[index]?.cue.id;
        if (targetId == null) return;
        api.moveCue(srcCueId, targetId);
      } else {
        api.moveCue(srcCueId, null);
      }
      // showModel.moveCue(cue_id, newIndex);
    }
  };

  const click = (event: MouseEvent, index: number) => {
    if (event.button != 0) {
      return;
    }
    const clickedId = showModel.flatCueList[index]?.cue.id;
    if (clickedId == null) return;
    if (event.shiftKey) {
      if (uiState.selected != null) {
        uiState.selectedRows = [];
        const prevIndex = showModel.flatCueList.findIndex((item) => item.cue.id === uiState.selected);
        if (index >= prevIndex) {
          for (let i = prevIndex; i <= index; i++) {
            const targetCueId = showModel.flatCueList[i]?.cue.id;
            if (targetCueId == null) continue;
            uiState.selectedRows.push(targetCueId);
          }
        } else {
          for (let i = index; i <= prevIndex; i++) {
            const targetCueId = showModel.flatCueList[i]?.cue.id;
            if (targetCueId == null) continue;
            uiState.selectedRows.push(targetCueId);
          }
        }
        uiState.selected = clickedId;
        uiState.setPlaybackCursor(clickedId);
      } else {
        uiState.setSelected(clickedId);
      }
    } else if (event.ctrlKey) {
      if (uiState.selected != null) {
        if (uiState.selectedRows.includes(clickedId)) {
          uiState.removeFromSelected(clickedId);
        } else {
          uiState.addSelected(clickedId);
        }
      } else {
        uiState.setSelected(clickedId);
      }
    } else {
      uiState.setSelected(clickedId);
    }
  };

  // const resetSelection = (event: MouseEvent): void => {
  //   if (event.button != 0) {
  //     return;
  //   }
  //   uiState.clearSelected();
  // };

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

  const getStatus = (id: string): string => {
    if (showState.activeCues[id] == undefined) {
      return '';
    }
    return showState.activeCues[id].status;
  };

  const openEditable = (e: MouseEvent, rowIndex: number, editType: string) => {
    if (uiState.mode != 'edit') {
      return;
    }
    if (e.target == null || !(e.target instanceof HTMLElement) || e.target.contentEditable === 'true') {
      return;
    }
    const targetCue = showModel.flatCueList[rowIndex];
    if (targetCue == null) return;
    if (editType == 'cuelist_duration') {
      const cueType = targetCue.cue.params.type;
      if (cueType != 'wait' && cueType != 'fade') {
        return;
      }
    }
    if (editType == 'cuelist_post_wait') {
      if (targetCue.isSequenceOverrided) {
        return;
      }
      if (targetCue.sequence.type != 'autoContinue') {
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
      const targetCue = showModel.flatCueList[rowIndex];
      if (targetCue == null) return;
      const newCue = structuredClone(toRaw(targetCue.cue));
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
      api.updateCue(newCue);
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
      (['Playing', 'Paused', 'Stopping', 'Completed'] as PlaybackStatus[]).includes(
        showState.activeCues[cue_id]!.status,
      )
    );
  };

  const setPlaybackCursor = (cueId: string) => {
    if (!getLockCursorToSelection()) {
      api.setPlaybackCursor(cueId);
    }
  };
</script>

<style lang="css" module>
  .cuelist {
    table {
      table-layout: fixed;
      font-size: 0.9em;
      min-width: 920px;
    }
    > div {
      scroll-padding-top: 34px;
    }
  }
  .drag-over-row > td {
    border-top: 2px solid rgb(var(--v-theme-primary)) !important;
  }
  .selected-row > td {
    background-color: rgb(var(--v-theme-primary), 0.2);
  }
</style>
