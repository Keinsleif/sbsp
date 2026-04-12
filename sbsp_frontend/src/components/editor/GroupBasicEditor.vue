<template>
  <v-sheet
    flat
    class="d-flex flex-column pa-4 ga-4"
  >
    <v-select
      v-model="mode"
      hide-details
      persistent-placeholder
      :label="t('main.bottomEditor.group.mode.label')"
      :items="[
        { value: 'playlist', name: t('main.bottomEditor.group.mode.playlist') },
        { value: 'concurrency', name: t('main.bottomEditor.group.mode.concurrency') },
        { value: 'startFirst', name: 'Start First' },
      ]"
      item-value="value"
      item-title="name"
      variant="outlined"
      density="compact"
      autocomplete="off"
      @update:model-value="saveEditorValue"
      @keydown.stop
    />
    <v-checkbox
      v-show="selectedCue != null && mode == 'playlist'"
      v-model="repeat"
      hide-details
      density="compact"
      :label="t('main.bottomEditor.timeLevels.repeat')"
      @update:model-value="saveEditorValue"
    />
    <v-checkbox
      v-show="selectedCue != null && mode == 'startFirst'"
      v-model="enter"
      hide-details
      density="compact"
      :label="'Advance cursor into Group'"
      @update:model-value="saveEditorValue"
    />
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
  selectedCue.value != null
  && selectedCue.value.params.type == 'group'
  && selectedCue.value.params.mode.type == 'playlist'
    ? selectedCue.value.params.mode.repeat
    : null,
);

const enter = ref(
  selectedCue.value != null
  && selectedCue.value.params.type == 'group'
  && selectedCue.value.params.mode.type == 'startFirst'
    ? selectedCue.value.params.mode.enter
    : null,
);

watch(selectedCue, () => {
  if (selectedCue.value == null || selectedCue.value.params.type != 'group') {
    return;
  }
  mode.value = selectedCue.value.params.mode.type;
  repeat.value = selectedCue.value.params.mode.type == 'playlist' ? selectedCue.value.params.mode.repeat : null;
  enter.value = selectedCue.value.params.mode.type == 'startFirst' ? selectedCue.value.params.mode.enter : null;
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
      } else if (selectedCue.value.params.mode.type == 'startFirst') {
        selectedCue.value.params.mode.enter = false;
      }
    }
    if (
      selectedCue.value.params.mode.type == 'playlist'
      && repeat.value != null
      && repeat.value != selectedCue.value.params.mode.repeat
    ) {
      selectedCue.value.params.mode.repeat = repeat.value;
    }
    if (
      selectedCue.value.params.mode.type == 'startFirst'
      && enter.value != null
      && enter.value != selectedCue.value.params.mode.enter
    ) {
      selectedCue.value.params.mode.enter = enter.value;
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
