<template>
  <v-sheet flat class="d-flex flex-row pa-4 ga-4">
    <v-sheet flat class="d-flex flex-column ga-2 flex-grow-0 flex-shrink-0" width="175px">
      <text-input v-model="number" class="flex-grow-0" :label="t('main.number')" @update="saveEditorValue"></text-input>
      <time-input
        class="flex-grow-0"
        v-model="duration"
        :disabled="selectedCue != null && selectedCue.params.type != 'wait' && selectedCue.params.type != 'fade'"
        :label="t('main.duration')"
        @update="saveEditorValue"
      ></time-input>
      <time-input
        v-model="preWait"
        class="flex-grow-0"
        :label="t('main.preWait')"
        @update="saveEditorValue"
      ></time-input>
      <v-select
        class="flex-grow-0"
        hide-details
        persistent-placeholder
        v-model="sequence"
        :label="t('main.continueMode.title')"
        :disabled="(selectedCue != null && selectedCue.id in showState.activeCues) || props.sequenceOverride != null"
        ref="cue_sequence"
        :items="[
          { value: 'doNotContinue', name: t('main.continueMode.doNotContinue') },
          { value: 'autoContinue', name: t('main.continueMode.autoContinue') },
          { value: 'autoFollow', name: t('main.continueMode.autoFollow') },
        ]"
        item-value="value"
        item-title="name"
        variant="outlined"
        density="compact"
        autocomplete="off"
        @update:modelValue="saveEditorValue"
        @keydown.stop
      ></v-select>
    </v-sheet>
    <v-sheet flat class="d-flex flex-grow-1 flex-shrink-1 flex-column ga-2">
      <text-input
        :placeholder="selectedCue != null ? buildCueName(selectedCue) : ''"
        v-model="name"
        :label="t('main.name')"
        alignInput="left"
        class="flex-grow-0"
        @update="saveEditorValue"
      ></text-input>
      <text-input
        class="flex-grow-1 flex-shrink-1"
        v-model="notes"
        :label="t('main.notes')"
        textType="area"
        @update="saveEditorValue"
      ></text-input>
      <v-sheet flat class="d-flex flex-row flex-grow-0 flex-shrink-0 ga-3">
        <time-input
          width="175px"
          v-model="postWait"
          class="flex-grow-0"
          :disabled="
            (selectedCue != null && selectedCue.id in showState.activeCues && sequence != 'autoContinue') ||
            props.sequenceOverride != null
          "
          :label="t('main.postWait')"
          @update="saveEditorValue"
        ></time-input>
        <cue-select
          v-model="target"
          class="flex-grow-0"
          :label="t('main.bottomEditor.continueTargetCue')"
          cueType="all"
          :exclude="selectedCue?.id"
          :null-text="t('main.bottomEditor.basics.nextCue')"
          max-width="640px"
          :disabled="
            (selectedCue != null && selectedCue.id in showState.activeCues && sequence == 'doNotContinue') ||
            props.sequenceOverride != null
          "
          @update="saveEditorValue"
        />
        <v-btn
          class="ml-auto mr-0 flex-grow-0"
          density="compact"
          :disabled="selectedCue != null && !(selectedCue.id in showState.activeCues)"
          @click="insertTimestampToNote"
          >{{ t('main.bottomEditor.basics.timestamp') }}</v-btn
        >
      </v-sheet>
    </v-sheet>
  </v-sheet>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { buildCueName, getDuration, secondsToFormat } from '../../utils';
import TextInput from '../input/TextInput.vue';
import TimeInput from '../input/TimeInput.vue';
import type { Cue } from '../../types/Cue';
import { useShowState } from '../../stores/showstate';
import { useI18n } from 'vue-i18n';
import { NIL } from 'uuid';
import CueSelect from '../input/CueSelect.vue';
import type { CueSequence } from '../../types/CueSequence';

const { t } = useI18n();

const showState = useShowState();

const selectedCue = defineModel<Cue | null>();
const props = withDefaults(
  defineProps<{
    sequenceOverride?: CueSequence | null;
  }>(),
  {
    sequenceOverride: null,
  },
);
const emit = defineEmits(['update']);

const overridedSequence = computed(() =>
  props.sequenceOverride != null
    ? props.sequenceOverride
    : selectedCue.value != null
      ? selectedCue.value.sequence
      : null,
);

const number = ref(selectedCue.value != null ? selectedCue.value.number : null);
const duration = ref(getDuration(selectedCue.value));
const preWait = ref(selectedCue.value != null ? selectedCue.value.preWait : null);
const sequence = ref(overridedSequence.value != null ? overridedSequence.value.type : null);
const postWait = ref(
  overridedSequence.value != null && overridedSequence.value.type != 'doNotContinue'
    ? overridedSequence.value.type == 'autoContinue'
      ? overridedSequence.value.postWait
      : getDuration(selectedCue.value)
    : null,
);
const name = ref(selectedCue.value != null ? selectedCue.value.name : null);
const notes = ref(selectedCue.value != null ? selectedCue.value.notes : null);
const target = ref(
  overridedSequence.value != null &&
    overridedSequence.value.type != 'doNotContinue' &&
    overridedSequence.value.targetId != NIL
    ? overridedSequence.value.targetId
    : null,
);

watch(selectedCue, () => {
  number.value = selectedCue.value != null ? selectedCue.value.number : null;
  duration.value = getDuration(selectedCue.value);
  preWait.value = selectedCue.value != null ? selectedCue.value.preWait : null;
  sequence.value = overridedSequence.value != null ? overridedSequence.value.type : null;
  postWait.value =
    overridedSequence.value != null && overridedSequence.value.type != 'doNotContinue'
      ? overridedSequence.value.type == 'autoContinue'
        ? overridedSequence.value.postWait
        : getDuration(selectedCue.value)
      : null;
  name.value = selectedCue.value != null ? selectedCue.value.name : null;
  notes.value = selectedCue.value != null ? selectedCue.value.notes : null;
  target.value =
    overridedSequence.value != null && overridedSequence.value.type != 'doNotContinue'
      ? overridedSequence.value.targetId
      : null;
});

watch(
  () => getDuration(selectedCue.value),
  () => {
    const cueDuration = getDuration(selectedCue.value);
    duration.value = cueDuration;
    if (sequence.value == 'autoFollow') {
      postWait.value = cueDuration;
    }
  },
);

const saveEditorValue = () => {
  if (selectedCue.value == null) {
    return;
  }
  if (number.value != null) {
    selectedCue.value.number = number.value;
  }
  if (preWait.value != null) {
    selectedCue.value.preWait = preWait.value;
  }
  if (sequence.value != null && props.sequenceOverride == null) {
    selectedCue.value.sequence.type = sequence.value;
    if (selectedCue.value.sequence.type == 'doNotContinue') {
      target.value = null;
      postWait.value = null;
    } else {
      selectedCue.value.sequence.targetId = target.value != null ? target.value : null;
      if (selectedCue.value.sequence.type == 'autoContinue') {
        if (postWait.value != null) {
          selectedCue.value.sequence.postWait = postWait.value;
        } else {
          postWait.value = 0;
          selectedCue.value.sequence.postWait = 0;
        }
      } else {
        postWait.value = getDuration(selectedCue.value);
      }
    }
  }
  if (name.value != null) {
    const newName = name.value.trim();
    if (newName == '') {
      selectedCue.value.name = null;
    } else {
      selectedCue.value.name = newName;
    }
  }
  if (notes.value != null) {
    selectedCue.value.notes = notes.value;
  }
  emit('update');
};

const insertTimestampToNote = () => {
  if (selectedCue.value == null || !(selectedCue.value.id in showState.activeCues)) {
    return;
  }
  const activeCue = showState.activeCues[selectedCue.value.id];
  if (activeCue == null) {
    return;
  }
  if (notes.value != null && (notes.value.endsWith('\n') || notes.value == '')) {
    notes.value += `[${secondsToFormat(activeCue.position)}] `;
  } else {
    notes.value += `\n[${secondsToFormat(activeCue.position)}] `;
  }
  saveEditorValue();
};
</script>

<style lang="css" module>
.centered-input input {
  text-align: center;
}
</style>
