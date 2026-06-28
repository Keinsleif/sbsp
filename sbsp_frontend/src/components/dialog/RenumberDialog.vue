<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { computed, ref } from 'vue';
import { useUiState } from '../../stores/uiState.ts';
import { useI18n } from 'vue-i18n';
import { useApi } from '../../api';
import Dialog from 'primevue/dialog';
import ButtonWrapper from '../wrapper/ButtonWrapper.vue';
import NumberInput from '../input/NumberInput.vue';
import TextInput from '../input/TextInput.vue';

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
      startFrom.value || 0,
      increment.value || 1,
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
    class="w-auto"
    :header="t('dialog.renumber.title')"
    @keydown.stop="onKeydown"
    @contextmenu.prevent
  >
    <div class="flex flex-col gap-4 p-3 w-100 items-stretch">
      <number-input
        :min="0"
        :step="1"
        v-model="startFrom"
        show-buttons
        button-layout="horizontal"
        :label="t('dialog.renumber.startNumber')"
      />
      <number-input
        :min="1"
        :step="1"
        v-model="increment"
        show-buttons
        button-layout="horizontal"
        :label="t('dialog.renumber.increment')"
      />
      <text-input
        v-model="prefix"
        :label="t('dialog.renumber.prefix')"
      />
      <text-input
        v-model="suffix"
        :label="t('dialog.renumber.suffix')"
      />
      <text-input
        :model-value="preview"
        :label="t('dialog.renumber.preview')"
      />
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
