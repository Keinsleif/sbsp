<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { computed, onUnmounted, ref, toRaw, useTemplateRef } from 'vue';
import { useShowModel, type FlatCueEntry } from '../../stores/showModel';
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
import { AUDIO_EXTENSIONS, useApi } from '../../api';
import CueListRow from './CueListRow.vue';
import type { Cue } from '../../types/Cue';
import { useThrottleFn } from '@vueuse/core';
import { useHotkey } from '@/composables/useHotkey.ts';
import PathIcon from '../display/PathIcon.vue';
import ContextMenu from 'primevue/contextmenu';
import { getExtension, isUserTyping } from '@/utils.ts';
import CueListEmptyRow from './CueListEmptyRow.vue';
import type { InsertPosition } from '@/types/InsertPosition.ts';
import { useOsFileDrop } from '@/composables/useFileDrop.ts';
import { useToast } from 'primevue/usetoast';
import { useUiSettings } from '@/stores/uiSettings.ts';

const { t } = useI18n();
const api = useApi();
const toast = useToast();

const showModel = useShowModel();
const uiState = useUiState();
const uiSettings = useUiSettings();

const cueListBodyRef = useTemplateRef('cuelistBody');

const internalClipboard = ref<Cue[]>([]);

type RenderRow =
  | { kind: 'entry'; entry: FlatCueEntry; index: number }
  | { kind: 'end-slot'; parentId: string | null; level: number };

function buildRenderRows(flatList: FlatCueEntry[]): RenderRow[] {
  const rows: RenderRow[] = [];
  const stack: { entry: FlatCueEntry; level: number }[] = [];

  const flushStackAbove = (level: number) => {
    if (stack.length === 0) return;
    let stack_level = stack[stack.length - 1]?.level;
    while (stack_level != null && stack_level >= level) {
      const g = stack.pop();
      if (g != null) {
        rows.push({
          kind: 'end-slot',
          parentId: g.entry.cue.id,
          level: stack_level + 1,
        });
      }
      stack_level = stack[stack.length - 1]?.level;
    }
  };

  flatList.forEach((entry, index) => {
    if (entry.isHidden) return;

    flushStackAbove(entry.level);
    rows.push({ kind: 'entry', entry, index });

    if (entry.isGroup && entry.isExpanded) {
      stack.push({ entry, level: entry.level });
    }
  });

  flushStackAbove(0);

  rows.push({
    kind: 'end-slot',
    parentId: null,
    level: 0,
  });
  return rows;
}

const renderRows = computed(() => buildRenderRows(showModel.flatCueList));

const scrollIntoIndex = (index: number) => {
  if (cueListBodyRef.value != null && cueListBodyRef.value instanceof HTMLElement) {
    cueListBodyRef.value.children[index]?.scrollIntoView(false);
  }
};

const paste = () => {
  const cues: Cue[] = internalClipboard.value;

  if (cues.length > 0 && uiState.mode === 'edit') {
    if (uiState.selected != null) {
      api.addCues(cues, { type: 'after', target: uiState.selected });
    } else {
      api.addCues(cues, { type: 'inside', target: null, index: null });
    }
  }
};

const pasteHandler = (e: ClipboardEvent) => {
  if (isUserTyping(e)) return;
  paste();
};

const cut = () => {
  const cues = showModel.getSelectedCues;
  if (cues.length > 0) {
    internalClipboard.value = structuredClone(cues.map((cue) => toRaw(cue)));
    if (uiState.mode === 'edit') {
      api.removeCues(
        cues.map((cue) => cue.id),
        false,
      );
    }
  }
};

const cutHandler = (e: ClipboardEvent) => {
  if (isUserTyping(e)) return;
  cut();
};

const copy = () => {
  const cues = showModel.getSelectedCues;

  if (cues.length > 0) {
    internalClipboard.value = structuredClone(cues.map((cue) => toRaw(cue)));
  }
};

const copyHandler = (e: ClipboardEvent) => {
  if (isUserTyping(e)) return;
  copy();
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
    command: () => {
      if (uiState.mode === 'edit') {
        api.removeCues(Array.from(uiState.selectedRows));
      }
    },
  },
]);

const onArrowUp = useThrottleFn((e: KeyboardEvent, extend: boolean = false) => {
  const renderRowsCache = renderRows.value;
  if (uiState.selected != null) {
    let cursorIndex = renderRowsCache.findIndex(
      (item) => item.kind === 'entry' && item.entry.cue.id === uiState.selected,
    );
    let cursorCueRef = renderRowsCache[cursorIndex];
    if (cursorCueRef == null || cursorCueRef.kind !== 'entry') return;
    const origLevel = cursorCueRef.entry.level;

    cursorIndex--;
    cursorCueRef = renderRowsCache[cursorIndex];
    if (cursorCueRef == null) return;
    while (cursorCueRef.kind !== 'entry' || cursorCueRef.entry.isHidden) {
      cursorIndex--;
      cursorCueRef = renderRowsCache[cursorIndex];
      if (cursorCueRef == null) {
        return;
      }
    }
    if (extend) {
      if (cursorCueRef.entry.level !== origLevel) return;
      uiState.addSelected(cursorCueRef.entry.cue.id);
    } else {
      uiState.setSelected(cursorCueRef.entry.cue.id);
    }
    scrollIntoIndex(cursorIndex);
  } else {
    const firstRow = renderRowsCache[0];
    if (firstRow?.kind !== 'entry') return;
    uiState.setSelected(firstRow.entry.cue.id);
    scrollIntoIndex(0); // First cue cannot be Group child. This ensures visibility.
  }
}, 100);

useHotkey(
  () => uiSettings.settings.hotkey.edit.cuelistMoveUp,
  (e) => {
    e.preventDefault();
    onArrowUp(e);
  },
);
useHotkey(
  () => uiSettings.settings.hotkey.edit.cuelistExtendUp,
  (e) => {
    e.preventDefault();
    onArrowUp(e, true);
  },
);

const onArrowDown = useThrottleFn((e: KeyboardEvent, extend: boolean = false) => {
  const renderRowsCache = renderRows.value;
  if (uiState.selected != null) {
    let cursorIndex = renderRowsCache.findIndex(
      (item) => item.kind === 'entry' && item.entry.cue.id === uiState.selected,
    );
    let cursorCueRef = renderRowsCache[cursorIndex];
    if (cursorCueRef == null || cursorCueRef.kind !== 'entry') return;
    const origLevel = cursorCueRef.entry.level;

    cursorIndex++;
    cursorCueRef = renderRowsCache[cursorIndex];
    if (cursorCueRef == null) return;
    while (cursorCueRef.kind !== 'entry' || cursorCueRef.entry.isHidden) {
      cursorIndex++;
      cursorCueRef = renderRowsCache[cursorIndex];
      if (cursorCueRef == null) {
        return;
      }
    }
    if (extend) {
      if (cursorCueRef.entry.level !== origLevel) return;
      uiState.addSelected(cursorCueRef.entry.cue.id);
    } else {
      uiState.setSelected(cursorCueRef.entry.cue.id);
    }
    scrollIntoIndex(cursorIndex);
  } else {
    let lastVisibleIndex = renderRows.value.length - 1;
    let lastVisibleEntry = renderRows.value[lastVisibleIndex];
    if (lastVisibleEntry == null) return;
    while (lastVisibleEntry.kind !== 'entry' || lastVisibleEntry.entry.isHidden) {
      lastVisibleIndex--;
      lastVisibleEntry = renderRows.value[lastVisibleIndex];
      if (lastVisibleEntry == null) return;
    }
    uiState.setSelected(lastVisibleEntry.entry.cue.id);
    scrollIntoIndex(lastVisibleIndex);
  }
}, 100);

useHotkey(
  () => uiSettings.settings.hotkey.edit.cuelistMoveDown,
  (e) => {
    e.preventDefault();
    onArrowDown(e);
  },
);
useHotkey(
  () => uiSettings.settings.hotkey.edit.cuelistExtendDown,
  (e) => {
    e.preventDefault();
    onArrowDown(e, true);
  },
);

useHotkey(
  () => uiSettings.settings.hotkey.edit.selectAll,
  (e) => {
    e.preventDefault();
    // This operation not set uiState.selected. But selecting all will includes uiState.selected
    uiState.selectedRows.clear();
    showModel.flatCueList
      .filter((item) => !item.isHidden)
      .forEach((value) => uiState.selectedRows.add(value.cue.id));
  },
);

useHotkey(
  () => uiSettings.settings.hotkey.edit.delete,
  (e) => {
    e.preventDefault();
    if (uiState.mode === 'edit') {
      api.removeCues(Array.from(uiState.selectedRows));
    }
  },
);

const cuelistWrapperRef = useTemplateRef('cuelistWrapper');
const dragOverIndex = ref<number | null>(null);

type ReorderState = {
  pointerId: number;
  cueIds: string[];
  ghostEl: HTMLDivElement;
};
const reorderState = ref<ReorderState | null>(null);

const AUTO_SCROLL_EDGE = 48; // px
const AUTO_SCROLL_MAX_SPEED = 16; // px/frame
let autoScrollRAF: number | null = null;
let lastPointerX = 0;
let lastPointerY = 0;

const findRowIndexAtPoint = (x: number, y: number): number | null => {
  const body = cueListBodyRef.value;
  if (body == null) return null;
  const el = document.elementFromPoint(x, y);
  const tr = (el as HTMLElement | null)?.closest('tr');
  if (tr == null || tr.parentElement !== body) return null;
  const idx = Array.prototype.indexOf.call(body.children, tr);
  return idx === -1 ? null : idx;
};

const updateDragOverFromPoint = (x: number, y: number, excludeIds: string[] = []) => {
  const idx = findRowIndexAtPoint(x, y);
  const row = idx != null ? renderRows.value[idx] : null;
  const targetId = row?.kind === 'entry' ? row.entry.cue.id : null;

  if (idx == null || (targetId != null && excludeIds.includes(targetId))) {
    dragOverIndex.value = null;
  } else {
    dragOverIndex.value = idx;
  }
};

const runAutoScroll = () => {
  const wrapper = cuelistWrapperRef.value;
  if (wrapper == null || reorderState.value == null) {
    autoScrollRAF = null;
    return;
  }
  const rect = wrapper.getBoundingClientRect();
  if (lastPointerY < rect.top + AUTO_SCROLL_EDGE) {
    const ratio = Math.min(1, (rect.top + AUTO_SCROLL_EDGE - lastPointerY) / AUTO_SCROLL_EDGE);
    wrapper.scrollTop -= AUTO_SCROLL_MAX_SPEED * ratio;
  } else if (lastPointerY > rect.bottom - AUTO_SCROLL_EDGE) {
    const ratio = Math.min(1, (lastPointerY - (rect.bottom - AUTO_SCROLL_EDGE)) / AUTO_SCROLL_EDGE);
    wrapper.scrollTop += AUTO_SCROLL_MAX_SPEED * ratio;
  }
  updateDragOverFromPoint(lastPointerX, lastPointerY, reorderState.value.cueIds);
  autoScrollRAF = requestAnimationFrame(runAutoScroll);
};

const ensureAutoScroll = () => {
  if (autoScrollRAF == null) autoScrollRAF = requestAnimationFrame(runAutoScroll);
};
const stopAutoScroll = () => {
  if (autoScrollRAF != null) {
    cancelAnimationFrame(autoScrollRAF);
    autoScrollRAF = null;
  }
};

const resolveDropTarget = (index: number | null): InsertPosition | null => {
  const row = index != null ? renderRows.value[index] : null;
  if (row == null) return null;
  if (row.kind === 'entry') return { type: 'before', target: row.entry.cue.id };
  if (row.kind === 'end-slot') return { type: 'inside', target: row.parentId, index: null };
  return null;
};

const onHandlePointerDown = (event: PointerEvent, cueId: string) => {
  if (uiState.mode !== 'edit' || event.button !== 0) return;
  event.stopPropagation();

  if (!uiState.selectedRows.has(cueId)) {
    uiState.setSelected(cueId);
  }
  const cueIds = Array.from(uiState.selectedRows);

  const ghostEl = document.createElement('div');
  ghostEl.textContent = cueIds.length > 0 ? t('main.cueList.movingCue', cueIds.length) : '';
  Object.assign(ghostEl.style, {
    position: 'fixed',
    left: `${event.clientX + 16}px`,
    top: `${event.clientY + 8}px`,
    padding: '2px 10px',
    borderRadius: '4px',
    background: 'var(--p-primary-color)',
    fontSize: '0.75em',
    pointerEvents: 'none',
    zIndex: '9999',
  } satisfies Partial<CSSStyleDeclaration>);
  document.body.appendChild(ghostEl);

  reorderState.value = { pointerId: event.pointerId, cueIds, ghostEl };
  (event.target as HTMLElement).setPointerCapture(event.pointerId);

  window.addEventListener('pointermove', onReorderPointerMove);
  window.addEventListener('pointerup', onReorderPointerUp);
  window.addEventListener('pointercancel', onReorderPointerCancel);
  document.addEventListener('keydown', onReorderKeydown);
  document.body.style.cursor = 'grabbing';

  lastPointerX = event.clientX;
  lastPointerY = event.clientY;
  updateDragOverFromPoint(lastPointerX, lastPointerY, reorderState.value.cueIds);
};

const onReorderPointerMove = (event: PointerEvent) => {
  const state = reorderState.value;
  if (state == null || event.pointerId !== state.pointerId) return;

  lastPointerX = event.clientX;
  lastPointerY = event.clientY;
  state.ghostEl.style.left = `${event.clientX + 16}px`;
  state.ghostEl.style.top = `${event.clientY + 8}px`;

  const wrapper = cuelistWrapperRef.value;
  if (wrapper != null) {
    const rect = wrapper.getBoundingClientRect();
    if (
      event.clientY < rect.top + AUTO_SCROLL_EDGE ||
      event.clientY > rect.bottom - AUTO_SCROLL_EDGE
    ) {
      ensureAutoScroll();
    } else {
      stopAutoScroll();
    }
  }
  updateDragOverFromPoint(event.clientX, event.clientY, reorderState.value?.cueIds);
};

const finishReorder = (commit: boolean) => {
  const state = reorderState.value;
  if (state == null) return;

  stopAutoScroll();
  window.removeEventListener('pointermove', onReorderPointerMove);
  window.removeEventListener('pointerup', onReorderPointerUp);
  window.removeEventListener('pointercancel', onReorderPointerCancel);
  document.removeEventListener('keydown', onReorderKeydown);
  document.body.style.cursor = '';
  state.ghostEl.remove();

  if (commit && dragOverIndex.value != null && uiState.mode === 'edit') {
    const target = resolveDropTarget(dragOverIndex.value);
    if (target != null) api.moveCues(state.cueIds, target);
  }

  reorderState.value = null;
  dragOverIndex.value = null;
};

const onReorderPointerUp = (event: PointerEvent) => {
  if (reorderState.value?.pointerId !== event.pointerId) return;
  finishReorder(true);
};
const onReorderPointerCancel = (event: PointerEvent) => {
  if (reorderState.value?.pointerId !== event.pointerId) return;
  finishReorder(false);
};
const onReorderKeydown = (event: KeyboardEvent) => {
  if (event.key === 'Escape') finishReorder(false);
};

useOsFileDrop({
  target: () => cuelistWrapperRef.value,
  onOver: (x, y) => {
    updateDragOverFromPoint(x, y);
  },
  onLeave: () => {
    dragOverIndex.value = null;
  },
  onDrop: (files, x, y) => {
    if (__IS_REMOTE__) return; // TODO: implement remote side behavier
    if (uiState.mode !== "edit") return;
    updateDragOverFromPoint(x, y);
    const target = resolveDropTarget(dragOverIndex.value);
    if (target == null) return;
    const acceptedPaths = new Set<string>();
    let invalidFileExists = false;
    for (const f of files) {
      if (f.kind === 'path') {
        const ext = getExtension(f.path);
        if (AUDIO_EXTENSIONS.includes(ext)) {
          acceptedPaths.add(f.path);
        } else {
          invalidFileExists = true;
        }
      } else {
        // TODO: implement file upload feature
      }
    }
    if (acceptedPaths.size > 0) {
      showModel.addAudioCueWithPath(Array.from(acceptedPaths), target);
    }
    if (invalidFileExists) {
      toast.add({
        severity: 'error',
        summary: t('notification.failedToAddCue'),
        detail: t('notification.invalidFileType'),
        life: 3000,
      });
    }
    dragOverIndex.value = null;
  },
});

const click = (event: MouseEvent, index: number) => {
  if (event.button !== 0) {
    return;
  }
  const row = renderRows.value[index];
  if (row == null || row.kind !== 'entry') return;
  const clickedId = row.entry.cue.id;
  if (clickedId == null) return;
  if (event.shiftKey) {
    if (uiState.selected != null) {
      // This operation manually add multiple cues and update playback cursor.
      uiState.selectedRows.clear();
      const prevIndex = renderRows.value.findIndex(
        (item) => item.kind !== 'end-slot' && item.entry.cue.id === uiState.selected,
      );
      const prevRow = renderRows.value[prevIndex];
      if (prevIndex === -1 || prevRow == null || prevRow.kind === 'end-slot') return;
      let lastSelected = null;
      if (index >= prevIndex) {
        for (let i = prevIndex; i <= index; i++) {
          const targetEntry = renderRows.value[i];
          if (
            targetEntry == null ||
            targetEntry.kind === 'end-slot' ||
            targetEntry.entry.isHidden ||
            targetEntry.entry.level !== prevRow.entry.level
          )
            continue;
          uiState.selectedRows.add(targetEntry.entry.cue.id);
          lastSelected = targetEntry.entry.cue.id;
        }
      } else {
        for (let i = prevIndex; i >= index; i--) {
          const targetEntry = renderRows.value[i];
          if (
            targetEntry == null ||
            targetEntry.kind === 'end-slot' ||
            targetEntry.entry.isHidden ||
            targetEntry.entry.level !== prevRow.entry.level
          )
            continue;
          uiState.selectedRows.add(targetEntry.entry.cue.id);
          lastSelected = targetEntry.entry.cue.id;
        }
      }
      uiState.selected = lastSelected;
      uiState.tryUpdatePlaybackCursor(lastSelected);
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

onUnmounted(() => {
  finishReorder(false);
});
</script>

<template>
  <div
    class="h-full scroll-pt-8 overflow-x-auto overflow-y-scroll"
    :class="$style['cuelist-wrapper']"
    tabindex="-1"
    ref="cuelistWrapper"
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
      <tbody ref="cuelistBody">
        <template
          v-for="(row, i) in renderRows"
          :key="row.kind === 'entry' ? row.entry.cue.id : `${row.parentId}-end`"
        >
          <cue-list-row
            v-if="row.kind === 'entry'"
            :item="row.entry"
            :is-drag-over="dragOverIndex === i"
            @handle-pointerdown="onHandlePointerDown($event, row.entry.cue.id)"
            @pointerdown.stop="click($event, i)"
            @contextmenu.prevent="
              if (uiState.mode == 'edit') {
                if (!uiState.selectedRows.has(row.entry.cue.id)) {
                  uiState.setSelected(row.entry.cue.id);
                }
                menu?.show($event);
              }
            "
          />
          <cue-list-empty-row
            v-else
            :parent-id="row.parentId"
            :level="row.level"
            :is-drag-over="dragOverIndex === i"
          />
        </template>
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
