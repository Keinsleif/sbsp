<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { computed, ref, watch } from 'vue';
import FadeParamInput from '../input/FadeParamInput.vue';
import { useShowState } from '../../stores/showState';
import type { Cue } from '../../types/Cue';
import type { FadeParam } from '../../types/FadeParam';
import CueSelect from '../input/CueSelect.vue';
import VolumeFader from '../input/VolumeFader.vue';
import { NIL } from 'uuid';
import { useI18n } from 'vue-i18n';
import { breakpointsTailwind, useBreakpoints } from '@vueuse/core';

const { t } = useI18n();
const breakpoints = useBreakpoints(breakpointsTailwind, { strategy: 'max-width' });
const xs = breakpoints.smaller('sm');
const smAndDown = breakpoints.smallerOrEqual('sm');
const showState = useShowState();

const selectedCue = defineModel<Cue | null>();
const emit = defineEmits(['update']);

const sliderChanging = ref(false);

const target = ref(
  selectedCue.value != null &&
    selectedCue.value.params.type === 'fade' &&
    selectedCue.value.params.target !== NIL
    ? selectedCue.value.params.target
    : '',
);

const volume = ref(
  selectedCue.value != null && selectedCue.value.params.type === 'fade'
    ? selectedCue.value.params.volume
    : 0,
);

const fadeParam = ref(
  selectedCue.value != null && selectedCue.value.params.type === 'fade'
    ? selectedCue.value.params.fadeParam
    : ({ duration: 3.0, easing: { type: 'inOutPow', intensity: 2 } } as FadeParam),
);

watch(selectedCue, () => {
  if (selectedCue.value == null || selectedCue.value.params.type !== 'fade') {
    return;
  }

  target.value = selectedCue.value.params.target;
  volume.value = selectedCue.value.params.volume;
  fadeParam.value = selectedCue.value.params.fadeParam;
});

const saveEditorValue = () => {
  if (selectedCue.value == null || sliderChanging.value === true) {
    return;
  }
  if (selectedCue.value.params.type !== 'fade') {
    return;
  }
  selectedCue.value.params.target = target.value;
  selectedCue.value.params.volume = volume.value;
  selectedCue.value.params.fadeParam = fadeParam.value;
  emit('update');
};

const isActive = computed(() => {
  return selectedCue.value != null && selectedCue.value.id in showState.activeCues;
});
</script>

<template>
  <div class="flex flex-col gap-3 p-3">
    <cue-select
      v-model="target"
      class="grow-0"
      :label="t('main.bottomEditor.targetCue')"
      :cue-type="['audio', 'group']"
      :disabled="isActive"
      @update="saveEditorValue"
    />
    <volume-fader
      v-model="volume"
      :label="t('main.bottomEditor.fade.targetVolume')"
      :disabled="isActive"
      :thumb-amount="smAndDown ? (xs ? 'baseOnly' : 'decreased') : 'full'"
      @update="saveEditorValue"
    />
    <fade-param-input
      v-model="fadeParam"
      class="self-start"
      :label="t('main.bottomEditor.fade.fadeParameter')"
      condition="both"
      disable-toggle
      :disabled="isActive"
      @update="saveEditorValue"
    />
  </div>
</template>
