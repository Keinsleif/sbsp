<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { useApi } from '@/api';
import { useUiState } from '@/stores/uiState';
import TreeIndentGuide from '../display/TreeIndentGuide.vue';

const props = defineProps<{
  parentId: string | null;
  isDragOver: boolean;
  level: number;
}>();

const api = useApi();
const uiState = useUiState();

const drop = (event: DragEvent) => {
  event.preventDefault();
  if (event.dataTransfer) {
    api.moveCues(Array.from(uiState.selectedRows), {
      type: 'inside',
      target: props.parentId,
      index: null,
    });
  }
};
</script>

<template>
  <tr
    :class="[$style['cue-row'], props.isDragOver ? $style['drag-over-row'] : '']"
    @drop="drop"
  >
    <td
      class="h-6"
      colspan="5"
    />
    <td class="h-6 text-slate-400">
      <tree-indent-guide
        :level="props.level"
        type="end"
      />
      <span class="pl-2">{{ props.level > 0 ? 'end' : 'end of list' }}</span>
    </td>
    <td
      class="h-6"
      colspan="4"
    ></td>
  </tr>
</template>

<style lang="css" module>
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
</style>
