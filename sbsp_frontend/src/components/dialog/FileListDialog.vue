<template>
  <v-dialog v-model="isFileListDialogOpen" width="640" height="360" @keydown.stop @contextmenu.prevent>
    <v-sheet class="d-flex flex-column w-100 h-100 py-1">
      <h3 class="pa-2 border-b-thin">{{ t('dialog.fileSelect.title') }}</h3>
      <v-treeview
        class="flex-grow-1 overflow-y-auto"
        :items="fileList"
        density="compact"
        item-children="files"
        item-title="name"
        item-value="path"
        open-on-click
        color="primary"
        selectable
        :select-strategy="props.multiple ? 'leaf' : 'single-leaf'"
        v-model:selected="selected"
      >
        <template v-slot:item="props">
          <v-treeview-item
            v-show="props.internalItem.raw.type == 'file' ? extList.includes(props.internalItem.raw.extension) : false"
            v-bind="props.props"
          ></v-treeview-item>
        </template>
      </v-treeview>
      <v-footer class="flex-grow-0 d-flex align-center ml-0 mr-0 w-100 ga-3">
        <v-btn class="ml-auto" :text="t('general.cancel')" variant="outlined" @click="pickFile(null)"></v-btn>
        <v-btn
          :disabled="selected.length == 0"
          :text="t('general.open')"
          color="primary"
          @click="pickFile(selected)"
        ></v-btn>
      </v-footer>
    </v-sheet>
  </v-dialog>
</template>

<script setup lang="ts">
  import { computed, ref, watch } from 'vue';
  import type { FileList } from '../../types/FileList.ts';
  import { useI18n } from 'vue-i18n';
  import { useApi } from '../../api/index.ts';

  const { t } = useI18n();
  const api = useApi();
  const fileListResolver = defineModel<((fileList: string[] | null) => void) | null>();
  const props = withDefaults(
    defineProps<{
      multiple: boolean;
    }>(),
    {
      multiple: true,
    },
  );
  const isFileListDialogOpen = computed(() => {
    return fileListResolver.value != null;
  });

  const pickFile = (select: string[] | null) => {
    if (fileListResolver.value != null) {
      fileListResolver.value(select);
      fileListResolver.value = null;
    }
  };

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

  let unlisten: (() => void) | null = null;

  watch(isFileListDialogOpen, (value) => {
    if (value) {
      api.setTitle(t('dialog.fileSelect.title'));
      api.remote
        ?.onFileListUpdate((list) => {
          fileList.value = list;
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
