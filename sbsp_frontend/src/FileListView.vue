<template>
  <div class="d-flex flex-column w-100 h-100">
    <v-treeview
      class="flex-grow-1"
      :items="fileList"
      density="compact"
      item-children="files"
      item-title="name"
      item-value="path"
      open-on-click
      color="primary"
      selectable
      select-strategy="leaf"
      v-model:selected="selected"
    >
      <template v-slot:item="props">
        <v-treeview-item
          v-if="props.internalItem.raw.type == 'file' ? extList.includes(props.internalItem.raw.extension) : false"
          v-bind="props.props"
        ></v-treeview-item>
      </template>
    </v-treeview>
    <v-footer class="flex-grow-0 d-flex align-center ml-0 mr-0 w-100 ga-3">
      <v-btn class="ml-auto" :text="t('general.cancel')" variant="outlined" @click="returnResult(null)"></v-btn>
      <v-btn
        :disabled="selected.length == 0"
        :text="t('general.open')"
        color="primary"
        @click="returnResult(selected)"
      ></v-btn>
    </v-footer>
  </div>
</template>

<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { onMounted, onUnmounted, ref } from 'vue';
import type { FileList } from './types/FileList.ts';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();

const extList = [
  'aiff',
  'aif',
  'caf',
  'mp4',
  'm4a',
  'mkv',
  'mka',
  'webm',
  'ogg',
  'oga',
  'wav',
  'aac',
  'alac',
  'flac',
  'mp3',
];

const fileList = ref<FileList[]>([]);
const selected = ref<string[]>([]);

const returnResult = (path: string[] | null) => {
  const appWebview = getCurrentWebviewWindow();
  appWebview.emitTo(appWebview.label, 'file-select-result', path);
};

let unlisten: UnlistenFn | null = null;

onMounted(() => {
  getCurrentWebviewWindow().setTitle(t('view.fileSelector.title'));
  listen<FileList[]>('asset-list-update', (event) => {
    fileList.value = event.payload;
  }).then((unlisten_func) => {
    unlisten = unlisten_func;
  });
  invoke('request_file_list').catch((e) => console.error(e));
});

onUnmounted(() => {
  if (unlisten != null) {
    unlisten();
  }
});
</script>
