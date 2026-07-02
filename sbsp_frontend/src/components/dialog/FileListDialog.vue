<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { computed, ref, watch } from 'vue';
import type { FileList } from '../../types/FileList.ts';
import { useI18n } from 'vue-i18n';
import { AUDIO_EXTENSIONS, useApi } from '../../api/index.ts';
import Dialog from 'primevue/dialog';
import type { TreeNode } from 'primevue/treenode';
import TreeTable from 'primevue/treetable';
import Column from 'primevue/column';
import ButtonWrapper from '../wrapper/ButtonWrapper.vue';

const { t } = useI18n();
const api = useApi();
const fileListResolver = defineModel<((fileList: string[] | null) => void) | null>();
const props = withDefaults(
  defineProps<{
    multiple?: boolean;
  }>(),
  {
    multiple: true,
  },
);

const isFileListDialogOpen = computed({
  get() {
  return fileListResolver.value != null;
},
  set(newValue) {
    if (!newValue && fileListResolver.value != null) {
      fileListResolver.value(null);
      fileListResolver.value = null;
    }
  },
});

const pickFile = (select: string[] | null) => {
  if (fileListResolver.value != null) {
    fileListResolver.value(select);
    fileListResolver.value = null;
  }
};

const fileList = ref<FileList[]>([]);
const selected = ref<{ [key: number]: unknown}>({});
const pathMap = new Map<string, string>();

const transformToTreeNodes = (list: FileList[], parentKey = ''): TreeNode[] => {
  return list
    .filter((item) => item.type === 'dir' || AUDIO_EXTENSIONS.includes(item.extension))
    .map((item) => {
      const currentKey = parentKey ? `${parentKey}-${item.name}` : `${item.name}`;

      if (item.type === 'dir') {
        return {
          key: currentKey,
          label: item.name,
          data: item,
          selectable: false,
          children: transformToTreeNodes(item.files, currentKey),
        };
      } else {
        pathMap.set(currentKey, item.path);
        return {
          key: currentKey,
          label: item.name,
          data: item,
          selectable: true,
        };
      }
    }).filter((item) => item.children == null || item.children.length > 0);
};

const treeValue = computed(() => transformToTreeNodes(fileList.value));

let unlisten: (() => void) | null = null;

watch(isFileListDialogOpen, (value) => {
  if (value) {
    api.remote
      ?.onFileListUpdate((list) => {
        fileList.value = list;
        selected.value = {};
      })
      .then((unlisten_func) => {
        unlisten = unlisten_func;
      });
    api.remote?.requestFileList();
  } else {
    if (unlisten != null) {
      unlisten();
    }
  }
});
</script>

<template>
  <Dialog
    v-model:visible="isFileListDialogOpen"
    class="h-120 w-160"
    :header="t('dialog.fileSelect.title')"
    @keydown.stop
    @contextmenu.prevent
  >
    <tree-table
      v-model:selectionKeys="selected"
      scroll-height="flex"
      :value="treeValue"
      color="primary"
      scrollable
      :selection-mode="props.multiple ? 'multiple' : 'single'"
    >
      <Column field="name" header="Name" expander />
    </tree-table>
    <template #footer>
      <button-wrapper
        :disabled="Object.keys(selected).length == 0"
        :label="t('general.open')"
        severity="primary"
        @click="pickFile(Object.keys(selected).map((key) => pathMap.get(key)).filter(item => item != null))"
      />
    </template>
  </Dialog>
</template>
