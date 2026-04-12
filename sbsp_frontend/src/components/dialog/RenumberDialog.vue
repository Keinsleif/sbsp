<template>
  <v-dialog
    v-model="isRenumberDialogOpen"
    width="auto"
    @keydown.enter.stop="onDone"
    @keydown.esc.stop="isRenumberDialogOpen = false"
    @keydown.stop
    @contextmenu.prevent
  >
    <v-sheet
      class="d-flex flex-column ga-4 pa-3"
      width="400px"
    >
      <h2>{{ t('dialog.renumber.title') }}</h2>
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
    </v-sheet>
  </v-dialog>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue';
import { useUiState } from '../../stores/uistate';
import { useI18n } from 'vue-i18n';
import { useApi } from '../../api';
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

const onDone = () => {
  api
    .renumberCues(uiState.selectedRows, startFrom.value, increment.value, prefix.value.trim() || null, suffix.value.trim() || null)
    .then(() => {
      isRenumberDialogOpen.value = false;
    })
    .catch(e => console.error(e));
};
</script>
