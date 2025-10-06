<template>
  <div class="mt-3 pa-2 border-md rounded position-relative">
    <span class="position-absolute text-subtitle-1 text-medium-emphasis" style="left: 16px">{{ props.label }}</span>
    <v-sheet class="d-flex flex-row align-end ga-4">
      <v-checkbox
        hide-details
        :disabled="props.disabled"
        v-model="fadeEnabled"
        @update:model-value="saveValues"
      ></v-checkbox>
      <time-input
        v-model="duration"
        :disabled="!fadeEnabled || props.disabled"
        class="flex-grow-0"
        label="Duration"
        width="100px"
        @update="saveValues"
      ></time-input>
      <v-select
        hide-details
        persistent-placeholder
        v-model="easingType"
        label="Curve"
        class="flex-grow-0"
        width="135px"
        :items="[
          { value: 'linear', name: 'Linear' },
          { value: 'inPow', name: 'InPow' },
          { value: 'outPow', name: 'OutPow' },
          { value: 'inOutPow', name: 'InOutPow' },
        ]"
        :disabled="!fadeEnabled || props.disabled"
        item-value="value"
        item-title="name"
        variant="outlined"
        density="compact"
        autocomplete="off"
        @update:model-value="saveValues"
        @keydown.stop
      ></v-select>
      <v-number-input
        hide-details
        inset
        persistent-placeholder
        v-model="easingPower"
        :disabled="!fadeEnabled || easingType == 'linear' || props.disabled"
        :min="1"
        width="160px"
        label="Intensity"
        density="compact"
        variant="outlined"
        autocomplete="off"
        @update:model-value="saveValues"
        @keydown.stop
      ></v-number-input>
      <curve-viewer
        :disabled="!fadeEnabled || props.disabled"
        class="border-md"
        width="80px"
        :type="props.condition"
        :curve="easingType"
        :power="easingPower"
      ></curve-viewer>
    </v-sheet>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue';
import { AudioCueFadeParam } from '../../types/AudioCueFadeParam';
import { curveToEasing, easingToCurve } from '../../utils';
import CurveViewer from './CurveViewer.vue';
import TimeInput from './TimeInput.vue';

const param = defineModel<AudioCueFadeParam | null>({ required: true });
const props = withDefaults(
  defineProps<{
    label?: string;
    condition?: 'in' | 'out';
    disabled?: boolean;
  }>(),
  {
    label: '',
    condition: 'in',
    disabled: false,
  },
);
const emit = defineEmits(['update']);

const fadeEnabled = ref<boolean>(param.value != null);
const duration = ref<number | null>(param.value != null ? param.value.duration : null);
const easingType = ref<'linear' | 'inPow' | 'outPow' | 'inOutPow' | null>(
  param.value != null ? easingToCurve(param.value.easing).type : null,
);
const easingPower = ref<number | null>(param.value != null ? easingToCurve(param.value.easing).power : null);

const saveValues = () => {
  if (fadeEnabled.value) {
    if (param.value == null) {
      param.value = {
        duration: 3,
        easing: { type: 'linear' },
      };
    } else {
      if (duration.value != null) {
        param.value.duration = duration.value;
      }
      if (easingType.value == 'linear') {
        easingPower.value = null;
      } else if (easingPower.value == null) {
        easingPower.value = 2;
      }
    }
    param.value.easing = curveToEasing({ type: easingType.value, power: easingPower.value });
  } else {
    param.value = null;
  }
  emit('update');
};
</script>
