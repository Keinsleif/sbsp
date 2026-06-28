<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { computed, ref, toRaw, useTemplateRef } from 'vue';
import { useShowModel } from '../../stores/showModel';
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
import { useUiState } from '../../stores/uiState';
import { useI18n } from 'vue-i18n';
import { useApi } from '../../api';
import CueListRow from './CueListRow.vue';
import type { Cue } from '../../types/Cue';
import { useThrottleFn } from '@vueuse/core';
import { useHotkey } from '@/composables/useHotkey.ts';
import PathIcon from '../display/PathIcon.vue';
import ContextMenu from 'primevue/contextmenu';
import { isUserTyping } from '@/utils.ts';

const { t } = useI18n();
const api = useApi();

const showModel = useShowModel();
const uiState = useUiState();

const cueListItemRefs = useTemplateRef('cuelistItem');

const internalClipboard = ref<Cue[]>([]);

const scrollIntoIndex = (index: number) => {
  if (cueListItemRefs.value != null) {
    cueListItemRefs.value[index]?.$el.scrollIntoView(false);
  }
};

const pasteHandler = (e: ClipboardEvent) => {
  if (isUserTyping(e)) return;

  const cues: Cue[] = internalClipboard.value;

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
    internalClipboard.value = structuredClone(cues.map((cue) => toRaw(cue)));
    api.removeCues(
      cues.map((cue) => cue.id),
      false,
    );
  }
};

const copyHandler = (e: ClipboardEvent) => {
  if (isUserTyping(e)) return;
  const cues = showModel.getSelectedCues;

  if (cues.length > 0) {
    e.preventDefault();
    internalClipboard.value = structuredClone(cues.map((cue) => toRaw(cue)));
  }
};

const paste = () => {
  const cues = internalClipboard.value;

  if (cues.length > 0) {
    api.addCues(cues, uiState.selected, false);
  }
};

const cut = () => {
  const cues = showModel.getSelectedCues;
  if (cues.length > 0) {
    internalClipboard.value = structuredClone(cues.map((cue) => toRaw(cue)));
    api.removeCues(
      cues.map((cue) => cue.id),
      false,
    );
  }
};

const copy = () => {
  const cues = showModel.getSelectedCues;

  if (cues.length > 0) {
    internalClipboard.value = structuredClone(cues.map((cue) => toRaw(cue)));
  }
};

const menu = useTemplateRef('menu');
const menuItems = computed(() => [
  { label: t('main.cueList.contextMenu.copy'), icon: mdiContentCopy, command: copy },
  { label: t('main.cueList.contextMenu.cut'), icon: mdiContentCut, command: cut },
  { label: t('main.cueList.contextMenu.paste'), icon: mdiContentPaste, command: paste },
  { separator: true },
  {
    label: t('main.cueList.contextMenu.delete'),
    icon: mdiTrashCan,
    command: () => api.removeCues(Array.from(uiState.selectedRows)),
  },
]);

const onArrowUp = useThrottleFn((e: KeyboardEvent) => {
  if (uiState.selected != null) {
    let cursorIndex =
      showModel.flatCueList.findIndex((item) => item.cue.id === uiState.selected) - 1;
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
      scrollIntoIndex(0); // First cue cannot be Group child. This ensures visibility.
    }
  }
}, 100);

useHotkey('ArrowUp', onArrowUp);
useHotkey('Shift+ArrowUp', onArrowUp);

const onArrowDown = useThrottleFn((e: KeyboardEvent) => {
  if (uiState.selected != null) {
    let cursorIndex =
      showModel.flatCueList.findIndex((item) => item.cue.id === uiState.selected) + 1;
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
    let lastVisibleIndex = showModel.flatCueList.length - 1;
    while (showModel.flatCueList[lastVisibleIndex]?.isHidden) {
      lastVisibleIndex--;
    }
    const lastCueId = showModel.flatCueList[lastVisibleIndex]?.cue.id;
    if (lastCueId != null) {
      uiState.setSelected(lastCueId);
      scrollIntoIndex(lastVisibleIndex);
    }
  }
}, 100);

useHotkey('ArrowDown', onArrowDown);
useHotkey('Shift+ArrowDown', onArrowDown);

useHotkey('$mod+A', () => {
  // This operation not set uiState.selected. But selecting all will includes uiState.selected
  uiState.selectedRows.clear();
  showModel.flatCueList
    .filter((item) => !item.isHidden)
    .forEach((value) => uiState.selectedRows.add(value.cue.id));
});

useHotkey('$mod+Backspace', () => {
  if (uiState.mode === 'edit') {
    api.removeCues(Array.from(uiState.selectedRows));
  }
});

const dragOverIndex = ref<number | null>(null);

const dragOver = (event: DragEvent, index: number, id: string) => {
  if (uiState.selectedRows.has(id)) {
    if (dragOverIndex.value != null) {
      dragOverIndex.value = null;
    }
  } else {
    event.preventDefault();
    if (dragOverIndex.value !== index) {
      dragOverIndex.value = index;
    }
  }
};

const dragEnd = () => {
  dragOverIndex.value = null;
};

const drop = (event: DragEvent) => {
  event.preventDefault();
  if (event.dataTransfer) {
    api.moveCues(Array.from(uiState.selectedRows), { type: 'last' });
  }
};

const click = (event: MouseEvent, index: number) => {
  if (event.button !== 0) {
    return;
  }
  const clickedId = showModel.flatCueList[index]?.cue.id;
  if (clickedId == null) return;
  if (event.shiftKey) {
    if (uiState.selected != null) {
      // This operation manually add multiple cues and update playback cursor.
      uiState.selectedRows.clear();
      const prevIndex = showModel.flatCueList.findIndex((item) => item.cue.id === uiState.selected);
      if (index >= prevIndex) {
        for (let i = prevIndex; i <= index; i++) {
          const targetCue = showModel.flatCueList[i];
          if (targetCue == null || targetCue.isHidden) continue;
          uiState.selectedRows.add(targetCue.cue.id);
        }
      } else {
        for (let i = index; i <= prevIndex; i++) {
          const targetCue = showModel.flatCueList[i];
          if (targetCue == null || targetCue.isHidden) continue;
          uiState.selectedRows.add(targetCue.cue.id);
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

<template>
  <div
    class="h-full scroll-pt-8 overflow-auto border border-(--p-form-field-border-color)"
    :class="$style['cuelist-wrapper']"
    tabindex="-1"
    @copy="copyHandler"
    @cut="cutHandler"
    @paste="pasteHandler"
  >
    <table
      class="w-full table-fixed border-separate border-spacing-0"
      :class="$style['cuelist']"
    >
      <thead>
        <tr>
          <th
            id="cuelist_handle"
            class="p-0"
            width="19px"
          />
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
            class="border-s border-(--p-form-field-border-color) text-center"
            width="54px"
          >
            #
          </th>
          <th
            id="cuelist_name"
            class="overflow-hidden border-x border-(--p-form-field-border-color) whitespace-nowrap"
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
            <div class="flex flex-row justify-center gap-1">
              {{ t('main.preWait') }}
              <path-icon
                class="mt-auto mb-auto cursor-pointer"
                :icon="
                  uiState.preWaitDisplayMode == 'elapsed'
                    ? mdiAlphaEBoxOutline
                    : mdiAlphaRBoxOutline
                "
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
            <div class="flex flex-row justify-center gap-1">
              {{ t('main.duration') }}
              <path-icon
                class="mt-auto mb-auto cursor-pointer"
                :icon="
                  uiState.durationDisplayMode == 'elapsed'
                    ? mdiAlphaEBoxOutline
                    : mdiAlphaRBoxOutline
                "
                @click.stop="uiState.toggleDurationDisplayMode"
              />
            </div>
          </th>
          <th
            id="cuelist_repeat"
            width="32px"
          >
            <path-icon :icon="mdiRepeat" />
          </th>
          <th
            id="cuelist_chain"
            width="54px"
          >
            <path-icon :icon="mdiChevronDoubleDown" />
          </th>
        </tr>
      </thead>
      <tbody>
        <cue-list-row
          v-for="(item, i) in showModel.flatCueList"
          v-show="!item.isHidden"
          ref="cuelistItem"
          :key="item.cue.id"
          :item="item"
          :is-drag-over="dragOverIndex == i"
          @dragover="dragOver($event, i, item.cue.id)"
          @dragend="dragEnd"
          @pointerdown.stop="click($event, i)"
          @contextmenu.prevent="
            if (uiState.mode == 'edit') {
              if (!uiState.selectedRows.has(item.cue.id)) {
                uiState.setSelected(item.cue.id);
              }
              menu?.show($event);
            }
          "
        />
        <tr
          :class="dragOverIndex == showModel.flatCueList.length ? $style['drag-over-row'] : ''"
          @dragover="dragOver($event, showModel.flatCueList.length, '')"
          @drop="drop"
        >
          <td
            colspan="10"
            class="border-b-0"
          />
        </tr>
      </tbody>
    </table>
  </div>
  <ContextMenu
    ref="menu"
    :model="menuItems"
  >
    <template #itemicon="innerProps">
      <path-icon
        v-if="innerProps.item.icon != null"
        :icon="innerProps.item.icon"
        :class="innerProps.class"
      ></path-icon>
    </template>
  </ContextMenu>
</template>

<style lang="css" module>
.cuelist-wrapper:focus {
  outline: none;
}

.cuelist {
  scroll-padding-top: 32px;
  font-size: 0.8em;
  min-width: 800px;
}

.cuelist th {
  position: sticky;
  background-color: color-mix(in oklab, var(--p-content-background) 60%, var(--p-surface-500));
  top: 0;
  z-index: 10;
}

@layer base {
  .cuelist th,
  .cuelist td {
    height: 32px;
    text-align: left;
    border-bottom: 1px solid var(--p-form-field-border-color);
  }
}

.drag-over-row > td {
  border-top: 2px solid var(--p-primary-color) !important;
}
</style>
