<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { computed, ref } from 'vue';
import { useUiState } from '../../stores/uiState';
import { useI18n } from 'vue-i18n';
import { useApi } from '../../api';
import Dialog from 'primevue/dialog';

const { t } = useI18n();
const api = useApi();
const uiState = useUiState();

const isRenumberDialogOpen = defineModel <boolean> ({ required: true });
const startFrom = ref(1);
const increment = ref(1);
const prefix = ref('');
const suffix = ref('');

const preview = computed(() => {
  let result = '';
  for (let i = 0; i < 3; i++) {
    result += prefix.value + (startFrom.value + increment.value * i) + suffix.value + '  ';
  }
  return result + '...';
});

const onKeydown = (e: KeyboardEvent) => {
  if (e.key === 'Enter') {
    onDone();
  } else if (e.key === 'Escape') {
    isRenumberDialogOpen.value = false;
  }
};

const onDone = () => {
  api
    .renumberCues(Array.from(uiState.selectedRows), startFrom.value, increment.value, prefix.value.trim() || null, suffix.value.trim() || null)
    .then(() => {
      isRenumberDialogOpen.value = false;
    })
    .catch(e => console.error(e));
};
</script>

<template>
  <Dialog
    v-model:visible="isRenumberDialogOpen"
    width="auto"
    :header="t('dialog.renumber.title')"
    @keydown.stop="onKeydown"
    @contextmenu.prevent
  >
    <div
      class="flex flex-col gap-4 p-3 w-100"
    >
      <v-number-input
        v-model="startFrom"
        persistent-placeholder
        hide-details
        :label="t('dialog.renumber.startNumber')"
        density="compact"
        variant="outlined"
      />
      <v-number-input
        v-model="increment"
        persistent-placeholder
        hide-details
        :label="t('dialog.renumber.increment')"
        density="compact"
        variant="outlined"
      />
      <text-input
        v-model="prefix"
        class="flex-grow-0"
        align-input="left"
        :label="t('dialog.renumber.prefix')"
      />
      <text-input
        v-model="suffix"
        class="flex-grow-0"
        align-input="left"
        :label="t('dialog.renumber.suffix')"
      />
      <text-input
        :model-value="preview"
        :label="t('dialog.renumber.preview')"
      />
      <v-sheet class="d-flex flex-row justify-end ga-2">
        <v-btn @click="isRenumberDialogOpen = false">
          {{ t('general.cancel') }}
        </v-btn>
        <v-btn
          color="primary"
          @click="onDone"
        >
          {{ t('general.done') }}
        </v-btn>
      </v-sheet>
    </div>
  </Dialog>
</template>