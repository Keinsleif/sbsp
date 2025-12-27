<template>
  <v-dialog
    v-model="isRenumberDialogOpen"
    width="auto"
    @keydown.enter.stop="onDone"
    @keydown.esc.stop="isRenumberDialogOpen = false"
    @keydown.stop
    @contextmenu.prevent
  >
    <v-sheet class="d-flex flex-column ga-4 pa-3" width="400px">
      <h2>{{ t('dialog.renumber.title') }}</h2>
      <v-number-input
        persistent-placeholder
        hide-details
        v-model="startFrom"
        :label="t('dialog.renumber.startNumber')"
        density="compact"
        variant="outlined"
      ></v-number-input>
      <v-number-input
        persistent-placeholder
        hide-details
        v-model="increment"
        :label="t('dialog.renumber.increment')"
        density="compact"
        variant="outlined"
      ></v-number-input>
      <v-sheet class="d-flex flex-row justify-end ga-2">
        <v-btn @click="isRenumberDialogOpen = false">{{ t('general.cancel') }}</v-btn>
        <v-btn color="primary" @click="onDone">{{ t('general.done') }}</v-btn>
      </v-sheet>
    </v-sheet>
  </v-dialog>
</template>

<script setup lang="ts">
import { ref } from 'vue';
import { useUiState } from '../../stores/uistate';
import { invoke } from '@tauri-apps/api/core';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();
const uiState = useUiState();

const isRenumberDialogOpen = defineModel<boolean>({ required: true });
const startFrom = ref(0);
const increment = ref(1);

const onDone = () => {
  invoke<{ cues: string[]; startFrom: number; increment: number }>('renumber_cues', {
    cues: uiState.selectedRows,
    startFrom: startFrom.value,
    increment: increment.value,
  })
    .then(() => {
      isRenumberDialogOpen.value = false;
    })
    .catch((e) => console.error(e));
};
</script>
