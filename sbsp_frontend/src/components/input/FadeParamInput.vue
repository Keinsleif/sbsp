<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { ref, watch } from 'vue';
import { curveToEasing, easingToCurve } from '../../utils';
import CurveViewer from '../display/CurveViewer.vue';
import TimeInput from './TimeInput.vue';
import { useI18n } from 'vue-i18n';
import type { FadeParam } from '../../types/FadeParam';
import SelectWrapper from '../wrapper/SelectWrapper.vue';
import CheckboxWrapper from '../wrapper/CheckboxWrapper.vue';
import NumberInput from './NumberInput.vue';

const { t } = useI18n();

const param = defineModel<FadeParam | null>({ required: true });
const props = withDefaults(
  defineProps<{
    label?: string;
    condition?: 'in' | 'out' | 'both';
    disableToggle?: boolean;
    disabled?: boolean;
  }>(),
  {
    label: '',
    condition: 'in',
    disableToggle: false,
    disabled: false,
  },
);
const emit = defineEmits(['update']);

const fadeEnabled = ref<boolean>(param.value != null || props.disableToggle);
const duration = ref<number | null>(param.value != null ? param.value.duration : null);
const easingType = ref<'linear' | 'inPow' | 'outPow' | 'inOutPow' | null>(
  param.value != null ? easingToCurve(param.value.easing).type : null,
);
const easingPower = ref<number | null>(
  param.value != null ? easingToCurve(param.value.easing).power : null,
);

watch(param, () => {
  if (param.value == null) {
    fadeEnabled.value = false;
    duration.value = null;
    easingType.value = null;
    easingPower.value = null;
  } else {
    fadeEnabled.value = true;
    duration.value = param.value.duration;
    const curve = easingToCurve(param.value.easing);
    easingType.value = curve.type;
    easingPower.value = curve.power;
  }
});

const saveValues = () => {
  if (fadeEnabled.value) {
    if (param.value == null) {
      param.value = {
        duration: 3,
        easing: { type: 'inOutPow', intensity: 2 },
      };
      duration.value = 3;
      easingType.value = 'inOutPow';
      easingPower.value = 2;
    } else {
      if (duration.value != null) {
        param.value.duration = duration.value;
      }
      if (easingType.value === 'linear') {
        easingPower.value = null;
      } else if (easingPower.value == null) {
        easingPower.value = 2;
      }
      param.value.easing = curveToEasing({ type: easingType.value, power: easingPower.value });
    }
  } else {
    param.value = null;
  }
  emit('update');
};
</script>

<template>
  <div class="p-2 border border-(--p-form-field-border-color) rounded relative">
    <span class="absolute top-1 left-2">
      {{ props.label }}
    </span>
    <div class="flex flex-row items-end gap-2">
      <checkbox-wrapper
        class="mb-2"
        v-show="!props.disableToggle"
        v-model="fadeEnabled"
        :disabled="props.disabled || props.disableToggle"
        @update:model-value="saveValues"
      />
      <time-input
        v-model="duration"
        :disabled="!fadeEnabled || props.disabled"
        class="grow-0 w-25"
        :label="t('main.duration')"
        @update="saveValues"
      />
      <select-wrapper
        v-model="easingType"
        :label="t('main.bottomEditor.input.curve')"
        class="grow-0 w-40"
        :items="[
          { value: 'linear', name: t('main.bottomEditor.input.linear') },
          { value: 'inPow', name: t('main.bottomEditor.input.easeIn') },
          { value: 'outPow', name: t('main.bottomEditor.input.easeOut') },
          { value: 'inOutPow', name: t('main.bottomEditor.input.easeInOut') },
        ]"
        :disabled="!fadeEnabled || props.disabled"
        autocomplete="off"
        @update:model-value="saveValues"
        @keydown.stop
      />
      <number-input
        v-model="easingPower"
        :disabled="!fadeEnabled || easingType == 'linear' || props.disabled"
        :min="1"
        :step="1"
        :label="t('main.bottomEditor.input.intensity')"
        show-buttons
        autocomplete="off"
        :max-fraction-digits="0"
        @update:model-value="saveValues"
        @keydown.stop
      />
      <curve-viewer
        :disabled="!fadeEnabled || props.disabled"
        class="w-16"
        :type="props.condition != 'out' ? 'in' : 'out'"
        :curve="easingType"
        :power="easingPower"
      />
      <curve-viewer
        v-if="props.condition == 'both'"
        :disabled="!fadeEnabled || props.disabled"
        class="w-16"
        type="out"
        :curve="easingType"
        :power="easingPower"
      />
    </div>
  </div>
</template>
