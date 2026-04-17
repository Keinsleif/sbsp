<template>
  <v-table
    fixed-header
    density="compact"
    class="flex-grow-1"
    :class="$style['cuelist']"
    height="100%"
  >
    <thead>
      <tr>
        <th
          id="cuelist_cursor"
          width="32px"
        />
        <th
          id="cuelist_status"
          width="32px"
        />
        <th
          id="cuelist_type"
          width="24px"
        />
        <th
          id="cuelist_number"
          class="text-center border-s"
          width="54px"
          style="padding: 0"
        >
          #
        </th>
        <th
          id="cuelist_name"
          class="border overflow-hidden text-no-wrap"
          style="padding-left: 24px"
        >
          {{ t('main.name') }}
        </th>
        <th
          id="cuelist_pre_wait"
          class="text-center"
          width="124px"
          style="padding: 0px 8px"
        >
          <div class="d-flex flex-row justify-center ga-1">
            {{ t('main.preWait') }}
            <v-icon
              class="mt-auto mb-auto"
              :icon="uiState.preWaitDisplayMode == 'elapsed' ? mdiAlphaEBoxOutline : mdiAlphaRBoxOutline"
              @click.stop="uiState.togglePreWaitDisplayMode"
            />
          </div>
        </th>
        <th
          id="cuelist_duration"
          class="text-center"
          width="124px"
          style="padding: 0px 8px"
        >
          <div class="d-flex flex-row justify-center ga-1">
            {{ t('main.duration') }}
            <v-icon
              class="mt-auto mb-auto"
              :icon="uiState.durationDisplayMode == 'elapsed' ? mdiAlphaEBoxOutline : mdiAlphaRBoxOutline"
              @click.stop="uiState.toggleDurationDisplayMode"
            />
          </div>
        </th>
        <th
          id="cuelist_repeat"
          width="32px"
        >
          <v-icon :icon="mdiRepeat" />
        </th>
        <th
          id="cuelist_chain"
          width="54px"
        >
          <v-icon :icon="mdiChevronDoubleDown" />
        </th>
      </tr>
    </thead>
    <tbody>
      <cue-list-row
        ref="cuelistItem"
        v-for="(item, i) in showModel.flatCueList"
        v-show="!item.isHidden"
        :key="item.cue.id"
        :item="item"
        :is-drag-over="dragOverIndex == i"
        @dragstart="dragStart($event, i)"
        @dragover="dragOver($event, i)"
        @dragend="dragEnd"
        @drop="drop($event, i)"
        @pointerdown.stop="click($event, i)"
      />
      <tr
        :class="dragOverIndex == showModel.flatCueList.length ? $style['drag-over-row'] : ''"
        @dragover="dragOver($event, showModel.flatCueList.length)"
        @drop="drop($event, showModel.flatCueList.length)"
      >
        <td colspan="9" />
      </tr>
    </tbody>
  </v-table>
</template>

<script setup lang="ts">
import { ref, useTemplateRef } from 'vue';
import { useShowModel } from '../../stores/showmodel';
import {
  mdiAlphaEBoxOutline,
  mdiAlphaRBoxOutline,
  mdiChevronDoubleDown,
  mdiRepeat,
} from '@mdi/js';
import { useUiState } from '../../stores/uistate';
import { useHotkey } from 'vuetify';
import { useI18n } from 'vue-i18n';
import { throttle } from 'vuetify/lib/util/throttle.mjs';
import { useApi } from '../../api';
import CueListRow from './CueListRow.vue';

const { t } = useI18n();
const api = useApi();

const showModel = useShowModel();
const uiState = useUiState();

const cueListItemRefs = useTemplateRef('cuelistItem');

const scrollIntoIndex = (index: number) => {
  if (cueListItemRefs.value != null) {
    cueListItemRefs.value[index]?.$el.scrollIntoView({ block: 'nearest' });
  }
};

const onArrowUp = throttle((e: KeyboardEvent) => {
  if (uiState.selected != null) {
    let cursorIndex = showModel.flatCueList.findIndex(item => item.cue.id == uiState.selected) - 1;
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
    let cursorIndex = showModel.flatCueList.findIndex(item => item.cue.id == uiState.selected) + 1;
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
  uiState.selectedRows = showModel.flatCueList.filter(item => !item.isHidden).map(item => item.cue.id);
});

useHotkey('cmd+backspace', () => {
  if (uiState.mode == 'edit') {
    for (const row of uiState.selectedRows) {
      api.removeCue(row);
    }
  }
});

const dragOverIndex = ref<number | null>(null);

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
      const prevIndex = showModel.flatCueList.findIndex(item => item.cue.id === uiState.selected);
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
</style>
