<template>
  <v-sheet flat class="d-flex flex-column pa-4 ga-4">
    <v-select
      hide-details
      persistent-placeholder
      v-model="mode"
      :label="t('main.bottomEditor.group.mode.label')"
      ref="cue_sequence"
      :items="[
        { value: 'playlist', name: t('main.bottomEditor.group.mode.playlist') },
        { value: 'concurrency', name: t('main.bottomEditor.group.mode.concurrency') },
      ]"
      item-value="value"
      item-title="name"
      :disabled="selectedCue!.id in showState.activeCues"
      variant="outlined"
      density="compact"
      autocomplete="off"
      @update:modelValue="saveEditorValue"
      @keydown.stop
    ></v-select>
  </v-sheet>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue';
import { useShowState } from '../../stores/showstate';
import type { Cue } from '../../types/Cue';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();
const showState = useShowState();

const selectedCue = defineModel<Cue | null>();
const emit = defineEmits(['update']);

const mode = ref(
  selectedCue.value != null && selectedCue.value.params.type == 'group' ? selectedCue.value.params.mode : null,
);

watch(selectedCue, () => {
  if (selectedCue.value == null || selectedCue.value.params.type != 'group') {
    return;
  }
  mode.value = selectedCue.value.params.mode;
});

const saveEditorValue = () => {
  if (selectedCue.value == null || selectedCue.value.params.type != 'group') {
    return;
  }
  if (mode.value != null) {
    selectedCue.value.params.mode = mode.value;
  }
  emit('update');
};
</script>

<style lang="css" module>
.centered-input input {
  text-align: center;
}
</style>
