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
      variant="outlined"
      density="compact"
      autocomplete="off"
      @update:modelValue="saveEditorValue"
      @keydown.stop
    ></v-select>
    <v-checkbox
      v-model="repeat"
      hide-details
      density="compact"
      :label="t('main.bottomEditor.timeLevels.repeat')"
      :disabled="selectedCue != null && mode != 'playlist'"
      @update:model-value="saveEditorValue"
    ></v-checkbox>
  </v-sheet>
</template>

<script setup lang="ts">
  import { ref, watch } from 'vue';
  import type { Cue } from '../../types/Cue';
  import { useI18n } from 'vue-i18n';

  const { t } = useI18n();

  const selectedCue = defineModel<Cue | null>();
  const emit = defineEmits(['update']);

  const mode = ref(
    selectedCue.value != null && selectedCue.value.params.type == 'group' ? selectedCue.value.params.mode.type : null,
  );

  const repeat = ref(
    selectedCue.value != null &&
      selectedCue.value.params.type == 'group' &&
      selectedCue.value.params.mode.type == 'playlist'
      ? selectedCue.value.params.mode.repeat
      : null,
  );

  watch(selectedCue, () => {
    if (selectedCue.value == null || selectedCue.value.params.type != 'group') {
      return;
    }
    mode.value = selectedCue.value.params.mode.type;
    repeat.value = selectedCue.value.params.mode.type == 'playlist' ? selectedCue.value.params.mode.repeat : null;
  });

  const saveEditorValue = () => {
    if (selectedCue.value == null || selectedCue.value.params.type != 'group') {
      return;
    }
    if (mode.value != null) {
      if (mode.value != selectedCue.value.params.mode.type) {
        selectedCue.value.params.mode.type = mode.value;
        if (selectedCue.value.params.mode.type == 'playlist') {
          selectedCue.value.params.mode.repeat = true;
        }
      }
      if (
        selectedCue.value.params.mode.type == 'playlist' &&
        repeat.value != null &&
        repeat.value != selectedCue.value.params.mode.repeat
      ) {
        selectedCue.value.params.mode.repeat = repeat.value;
      }
    }
    emit('update');
  };
</script>

<style lang="css" module>
  .centered-input input {
    text-align: center;
  }
</style>
