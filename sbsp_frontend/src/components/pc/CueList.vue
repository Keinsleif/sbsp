<template>
  <v-sheet
    class="d-flex h-100"
    :class="$style['cuelist-wrapper']"
    @copy="copyHandler"
    @cut="cutHandler"
    @paste="pasteHandler"
    tabindex="-1"
  >
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
          @click.stop="click($event, i)"
          @contextmenu.prevent="
            if (uiState.mode == 'edit') {
              contextMenuPosition = [$event.clientX, $event.clientY];
              isContextMenuOpen = true;
            }
          "
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
    <v-menu
      v-model="isContextMenuOpen"
      :target="contextMenuPosition || undefined"
      density="compact"
    >
      <v-list
        density="compact"
        @contextmenu.prevent
      >
        <v-list-item
          :title="t('main.cueList.contextMenu.copy')"
          density="compact"
          :disabled="uiState.mode != 'edit'"
          :prepend-icon="mdiContentCopy"
          @click="copy"
        />
        <v-list-item
          density="compact"
          :title="t('main.cueList.contextMenu.cut')"
          :disabled="uiState.mode != 'edit'"
          :prepend-icon="mdiContentCut"
          @click="cut"
        />
        <v-list-item
          density="compact"
          :title="t('main.cueList.contextMenu.paste')"
          :disabled="uiState.mode != 'edit'"
          :prepend-icon="mdiContentPaste"
          @click="paste"
        />
        <v-divider />
        <v-list-item
          density="compact"
          :title="t('main.cueList.contextMenu.delete')"
          :disabled="uiState.mode != 'edit'"
          :prepend-icon="mdiTrashCan"
          @click="api.removeCues(Array.from(uiState.selectedRows))"
        />
      </v-list>
    </v-menu>
  </v-sheet>
</template>

<script setup lang="ts">
import { ref, useTemplateRef } from 'vue';
import { useShowModel } from '../../stores/showmodel';
import {
  mdiAlphaEBoxOutline,
  mdiAlphaRBoxOutline,
  mdiChevronDoubleDown,
  mdiContentCopy,
  mdiContentCut,
  mdiContentPaste,
  mdiRepeat,
  mdiTrashCan,
} from '@mdi/js';
import { useUiState } from '../../stores/uistate';
import { useHotkey } from 'vuetify';
import { useI18n } from 'vue-i18n';
import { throttle } from 'vuetify/lib/util/throttle.mjs';
import { useApi } from '../../api';
import CueListRow from './CueListRow.vue';
import type { Cue } from '../../types/Cue';
// import { cueParser, cueStringify } from '../../typia';

const { t } = useI18n();
const api = useApi();

const showModel = useShowModel();
const uiState = useUiState();

const cueListItemRefs = useTemplateRef('cuelistItem');

const isContextMenuOpen = ref(false);
const contextMenuPosition = ref<[number, number] | null>(null);
const internalClipboard = ref<Cue[]>([]);

const scrollIntoIndex = (index: number) => {
  if (cueListItemRefs.value != null) {
    cueListItemRefs.value[index]?.$el.scrollIntoView({ block: 'nearest' });
  }
};

const isUserTyping = (e: ClipboardEvent): boolean => {
  const target = (e.target || document.activeElement) as HTMLElement | null;
  if (!target) return false;

  const tagName = target.tagName.toUpperCase();

  if (tagName === 'INPUT' || tagName === 'TEXTAREA') {
    return true;
  }

  if (target.closest('input, textarea')) {
    return true;
  }

  if (target.isContentEditable || target.closest('[contenteditable="true"]')) {
    return true;
  }

  return false;
};

const pasteHandler = (e: ClipboardEvent) => {
  if (isUserTyping(e)) return;

  let cues: Cue[] = [];
  // if (navigator.clipboard && e.clipboardData) {
  //   const rawText = e.clipboardData.getData('application/x-sbsp-cue');
  //   if (!rawText) return;
  //   cues = cueParser(rawText) || [];
  // } else {
  //   cues = internalClipboard.value;
  // }
  cues = internalClipboard.value;

  if (cues.length > 0) {
    e.preventDefault();
    api.addCues(cues, uiState.selected, true);
  }
};

const cutHandler = (e: ClipboardEvent) => {
  if (isUserTyping(e)) return;
  const cues = showModel.getSelectedCues;

  if (cues.length > 0) {
    e.preventDefault();
    internalClipboard.value = cues;
    api.removeCues(cues.map(cue => cue.id), false);
  }
};

const copyHandler = (e: ClipboardEvent) => {
  if (isUserTyping(e)) return;
  const cues = showModel.getSelectedCues;

  if (cues.length > 0) {
    e.preventDefault();
    internalClipboard.value = cues;
    // if (navigator.clipboard && e.clipboardData) {
    //   const text = cueStringify(cues);
    //   if (!text) return;
    //   e.clipboardData.setData('application/x-sbsp-cue', text);
    // } else {
    //   internalClipboard.value = cues;
    // }
  }
};

const paste = () => {
  let cues = internalClipboard.value;

  if (cues.length > 0) {
    api.addCues(cues, uiState.selected, false);
  }
};

const cut = () => {
  const cues = showModel.getSelectedCues;
  if (cues.length > 0) {
    internalClipboard.value = cues;
    api.removeCues(cues.map(cue => cue.id), false);
  }
};

const copy = () => {
  const cues = showModel.getSelectedCues;

  if (cues.length > 0) {
    internalClipboard.value = cues;
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
  uiState.selectedRows.clear();
  showModel.flatCueList.filter(item => !item.isHidden).forEach(value => uiState.selectedRows.add(value.cue.id));
});

useHotkey('cmd+backspace', () => {
  if (uiState.mode == 'edit') {
    api.removeCues(Array.from(uiState.selectedRows));
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
    console.log(uiState.selectedRows);
    // const srcCueId = showModel.flatCueList[fromIndex]?.cue.id;
    // if (srcCueId == undefined) return;
    if (index < showModel.flatCueList.length) {
      const targetId = showModel.flatCueList[index]?.cue.id;
      if (targetId == null) return;
      api.moveCues(Array.from(uiState.selectedRows), { type: 'before', target: targetId });
    } else {
      api.moveCues(Array.from(uiState.selectedRows), { type: 'last' });
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
      uiState.selectedRows.clear();
      const prevIndex = showModel.flatCueList.findIndex(item => item.cue.id === uiState.selected);
      if (index >= prevIndex) {
        for (let i = prevIndex; i <= index; i++) {
          const targetCueId = showModel.flatCueList[i]?.cue.id;
          if (targetCueId == null) continue;
          uiState.selectedRows.add(targetCueId);
        }
      } else {
        for (let i = index; i <= prevIndex; i++) {
          const targetCueId = showModel.flatCueList[i]?.cue.id;
          if (targetCueId == null) continue;
          uiState.selectedRows.add(targetCueId);
        }
      }
      uiState.selected = clickedId;
      uiState.setPlaybackCursor(clickedId);
    } else {
      uiState.setSelected(clickedId);
    }
  } else if (event.ctrlKey) {
    if (uiState.selected != null) {
      if (uiState.selectedRows.has(clickedId)) {
        uiState.removeFromSelected([clickedId]);
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
  .cuelist-wrapper:focus {
    outline: none;
  }
  .cuelist {
    table {
      table-layout: fixed;
      font-size: 0.9em;
      min-width: 800px;
    }
    > div {
      scroll-padding-top: 34px;
    }
  }
  .drag-over-row > td {
    border-top: 2px solid rgb(var(--v-theme-primary)) !important;
  }
</style>
