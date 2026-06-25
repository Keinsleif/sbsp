<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { computed, ref } from 'vue';
import { useUiState } from '../../stores/uiState.ts';
import { useI18n } from 'vue-i18n';
import { useApi } from '../../api';
import Dialog from 'primevue/dialog';
import ButtonWrapper from '../wrapper/ButtonWrapper.vue';
import InputNumber from 'primevue/inputnumber';
import FloatLabel from 'primevue/floatlabel';
import InputText from 'primevue/inputtext';

const { t } = useI18n();
const api = useApi();
const uiState = useUiState();

const isRenumberDialogOpen = defineModel<boolean>({ required: true });
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
    .renumberCues(
      Array.from(uiState.selectedRows),
      startFrom.value,
      increment.value,
      prefix.value.trim() || null,
      suffix.value.trim() || null,
    )
    .then(() => {
      isRenumberDialogOpen.value = false;
    })
    .catch((e) => console.error(e));
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
    <div class="flex flex-col gap-4 p-3 w-100 items-stretch">
      <float-label variant="on">
        <input-number
          id="start_from"
          class="w-full"
          :min="0"
          :step="1"
          v-model="startFrom"
          show-buttons
          button-layout="horizontal"
        />
        <label for="start_from">{{ t('dialog.renumber.startNumber') }}</label>
      </float-label>
      <float-label variant="on">
        <input-number
          id="increment"
          class="w-full"
          :min="0"
          :step="1"
          v-model="increment"
          show-buttons
          button-layout="horizontal"
        />
        <label for="increment">{{ t('dialog.renumber.increment') }}</label>
      </float-label>
      <float-label variant="on">
        <input-text
          class="w-full"
          id="prefix"
          v-model="prefix"
        />
        <label for="prefix">{{ t('dialog.renumber.prefix') }}</label>
      </float-label>
      <float-label variant="on">
        <input-text
          class="w-full"
          id="suffix"
          v-model="suffix"
        />
        <label for="suffix">{{ t('dialog.renumber.suffix') }}</label>
      </float-label>
      <float-label variant="on">
        <input-text
          class="w-full"
          id="preview"
          :model-value="preview"
          :label="t('dialog.renumber.preview')"
        />
        <label for="preview">{{ t('dialog.renumber.preview') }}</label>
      </float-label>
      <div class="flex flex-row justify-end">
        <button-wrapper
          :label="t('general.run')"
          severity="primary"
          @click="onDone"
        />
      </div>
    </div>
  </Dialog>
</template>
