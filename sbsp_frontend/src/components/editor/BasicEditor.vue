<template>
  <v-sheet
    flat
    class="d-flex flex-row pa-4 ga-4"
    min-width="720px"
  >
    <v-sheet
      flat
      class="d-flex flex-column ga-2 flex-grow-0 flex-shrink-0"
      width="175px"
    >
      <text-input
        v-model="number"
        class="flex-grow-0"
        :label="t('main.number')"
        @update="saveEditorValue"
      />
      <time-input
        v-model="duration"
        class="flex-grow-0"
        :disabled="selectedCue != null && selectedCue.params.type != 'wait' && selectedCue.params.type != 'fade'"
        :label="t('main.duration')"
        @update="saveEditorValue"
      />
      <time-input
        v-model="preWait"
        class="flex-grow-0"
        :label="t('main.preWait')"
        @update="saveEditorValue"
      />
      <v-select
        v-model="chain"
        class="flex-grow-0"
        hide-details
        persistent-placeholder
        :label="t('main.chainMode.title')"
        :disabled="(selectedCue != null && selectedCue.id in showState.activeCues) || props.chainOverride != null"
        :items="[
          { value: 'doNotChain', name: t('main.chainMode.doNotChain') },
          { value: 'afterStart', name: t('main.chainMode.afterStart') },
          { value: 'afterComplete', name: t('main.chainMode.afterComplete') },
        ]"
        item-value="value"
        item-title="name"
        variant="outlined"
        density="compact"
        autocomplete="off"
        @update:model-value="saveEditorValue"
        @keydown.stop
      />
    </v-sheet>
    <v-sheet
      flat
      class="d-flex flex-grow-1 flex-shrink-1 flex-column ga-2"
    >
      <text-input
        v-model="name"
        :placeholder="selectedCue != null ? buildCueName(selectedCue) : ''"
        :label="t('main.name')"
        align-input="left"
        class="flex-grow-0"
        @update="saveEditorValue"
      />
      <text-input
        v-model="notes"
        class="flex-grow-1 flex-shrink-1"
        :label="t('main.notes')"
        text-type="area"
        @update="saveEditorValue"
      />
      <v-sheet
        flat
        class="d-flex flex-row flex-grow-0 flex-shrink-0 ga-3"
      >
        <cue-select
          v-model="target"
          class="flex-grow-0"
          :label="t('main.bottomEditor.continueTargetCue')"
          cue-type="all"
          :exclude="selectedCue?.id"
          :null-text="t('main.bottomEditor.basics.nextCue')"
          max-width="640px"
          :disabled="props.chainOverride != null || selectedCue == null || selectedCue.id in showState.activeCues || chain == 'doNotChain'"
          @update="saveEditorValue"
        />
          <v-select
            v-model="color"
            class="ml-auto flex-grow-0"
            hide-details
            persistent-placeholder
            width="150px"
            :style="{'color': color != null && color != 'none' ? colors[color].base : ''}"
            :label="t('main.bottomEditor.basics.color')"
            :items="[
              {value: 'none', title: t('general.none'), props: {baseColor: 'text'}},
              {value: 'red', title: 'Red', props: {baseColor: 'red'}},
              {value: 'purple', title: 'Purple', props: {baseColor: 'purple'}},
              {value: 'blue', title: 'Blue', props: {baseColor: 'blue'}},
              {value: 'cyan', title: 'Cyan', props: {baseColor: 'cyan'}},
              {value: 'green', title: 'Green', props: {baseColor: 'green'}},
              {value: 'yellow', title: 'Yellow', props: {baseColor: 'yellow'}},
              {value: 'orange', title: 'Orange', props: {baseColor: 'orange'}},
              {value: 'grey', title: 'Grey', props: {baseColor: 'grey'}},
            ]"
            variant="outlined"
            :prepend-inner-icon="mdiCircle"
            density="compact"
            autocomplete="off"
            @update:model-value="saveEditorValue"
            @keydown.stop
          />
        <v-btn
          class="flex-grow-0"
          density="compact"
          :disabled="selectedCue != null && !(selectedCue.id in showState.activeCues)"
          @click="insertTimestampToNote"
        >
          {{ t('main.bottomEditor.basics.timestamp') }}
        </v-btn>
      </v-sheet>
    </v-sheet>
  </v-sheet>
</template>

<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { computed, ref, watch } from 'vue';
import { buildCueName, getDuration, secondsToFormat } from '../../utils';
import TextInput from '../input/TextInput.vue';
import TimeInput from '../input/TimeInput.vue';
import type { Cue } from '../../types/Cue';
import { useShowState } from '../../stores/showstate';
import { useI18n } from 'vue-i18n';
import { NIL } from 'uuid';
import CueSelect from '../input/CueSelect.vue';
import type { CueChain } from '../../types/CueChain';
import { mdiCircle } from '@mdi/js';
import colors from 'vuetify/util/colors';

const { t } = useI18n();

const showState = useShowState();

const selectedCue = defineModel<Cue | null>();
const props = withDefaults(
  defineProps<{
    chainOverride?: CueChain | null;
  }>(),
  {
    chainOverride: null,
  },
);
const emit = defineEmits(['update']);

const overridedChain = computed(() =>
  props.chainOverride != null
    ? props.chainOverride
    : selectedCue.value != null
      ? selectedCue.value.chain
      : null,
);

const number = ref(selectedCue.value != null ? selectedCue.value.number : null);
const duration = ref(getDuration(selectedCue.value));
const preWait = ref(selectedCue.value != null ? selectedCue.value.preWait : null);
const chain = ref(overridedChain.value != null ? overridedChain.value.type : null);

const name = ref(selectedCue.value != null ? selectedCue.value.name : null);
const notes = ref(selectedCue.value != null ? selectedCue.value.notes : null);
const color = ref(selectedCue.value != null ? selectedCue.value.color : null);
const target = ref(
  overridedChain.value != null
  && overridedChain.value.type != 'doNotChain'
  && overridedChain.value.targetId != NIL
    ? overridedChain.value.targetId
    : null,
);

watch(selectedCue, () => {
  number.value = selectedCue.value != null ? selectedCue.value.number : null;
  duration.value = getDuration(selectedCue.value);
  preWait.value = selectedCue.value != null ? selectedCue.value.preWait : null;
  chain.value = overridedChain.value != null ? overridedChain.value.type : null;
  name.value = selectedCue.value != null ? selectedCue.value.name : null;
  notes.value = selectedCue.value != null ? selectedCue.value.notes : null;
  color.value = selectedCue.value != null ? selectedCue.value.color : null;
  target.value
    = overridedChain.value != null && overridedChain.value.type != 'doNotChain'
      ? overridedChain.value.targetId
      : null;
});

watch(
  () => getDuration(selectedCue.value),
  () => {
    const cueDuration = getDuration(selectedCue.value);
    duration.value = cueDuration;
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
  if (chain.value != null && props.chainOverride == null) {
    selectedCue.value.chain.type = chain.value;
    if (selectedCue.value.chain.type == 'doNotChain') {
      target.value = null;
    } else {
      console.log(target.value);
      selectedCue.value.chain.targetId = target.value != null ? target.value : null;
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
  if (color.value != null) {
    selectedCue.value.color = color.value;
  }
  emit('update');
};

const insertTimestampToNote = () => {
  if (selectedCue.value == null) {
    return;
  }
  const position = showState.getPosition(selectedCue.value.id);
  if (position == null) return;

  if (notes.value == null) {
    notes.value = '';
  }

  if (notes.value.endsWith('\n') || notes.value == '') {
    notes.value += `[${secondsToFormat(position)}] `;
  } else {
    notes.value += `\n[${secondsToFormat(position)}] `;
  }
  saveEditorValue();
};
</script>

<style lang="css" module>
  .centered-input input {
    text-align: center;
  }
</style>
