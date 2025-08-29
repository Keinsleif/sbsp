<template>
  <div class="mt-3 pa-2 border-md rounded position-relative">
    <span class="position-absolute text-subtitle-1 text-medium-emphasis" style="left: 16px">{{ props.label }}</span>
    <v-sheet class="d-flex flex-row align-end ga-4">
      <v-checkbox hide-details v-model="fadeEnabled"></v-checkbox>
      <time-input
        v-model="duration"
        :disabled="!fadeEnabled"
        class="flex-grow-0"
        label="Duration"
        width="100px"
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
        :disabled="!fadeEnabled"
        item-value="value"
        item-title="name"
        variant="outlined"
        density="compact"
        autocomplete="off"
      ></v-select>
      <v-number-input
        hide-details
        inset
        persistent-placeholder
        v-model="easingPower"
        :disabled="!fadeEnabled || easingType == 'linear'"
        :min="1"
        width="160px"
        label="Intensity"
        density="compact"
        variant="outlined"
        autocomplete="off"
      ></v-number-input>
      <curve-viewer
        :disabled="!fadeEnabled"
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
import { computed } from 'vue';
import { AudioCueFadeParam } from '../../types/AudioCueFadeParam';
import { curveToEasing, easingToCurve } from '../../utils';
import CurveViewer from './CurveViewer.vue';
import TimeInput from './TimeInput.vue';

const param = defineModel<AudioCueFadeParam | null>({ required: true });
const props = withDefaults(
  defineProps<{
    label?: string;
    condition?: 'in' | 'out';
  }>(),
  {
    label: '',
    condition: 'in',
  },
);
const emit = defineEmits(['update']);

const fadeEnabled = computed({
  get() {
    return param.value != null;
  },
  set(newValue) {
    if (newValue) {
      param.value = {
        duration: 3,
        easing: { type: 'linear' },
      };
    } else {
      param.value = null;
    }
    document.body.focus();
    emit('update');
  },
});
const duration = computed({
  get() {
    return param.value != null ? param.value.duration : null;
  },
  set(newValue) {
    if (fadeEnabled.value && param.value != null && newValue != null) {
      param.value.duration = newValue;
      document.body.focus();
      emit('update');
    }
  },
});
const easingType = computed({
  get() {
    return param.value != null ? easingToCurve(param.value.easing).type : null;
  },
  set(newValue) {
    if (fadeEnabled.value && param.value != null && newValue != null) {
      param.value.easing = curveToEasing({ type: newValue, power: easingPower.value });
      document.body.focus();
      emit('update');
    }
  },
});
const easingPower = computed({
  get() {
    return param.value != null ? easingToCurve(param.value.easing).power : null;
  },
  set(newValue) {
    if (fadeEnabled.value && param.value != null && easingType.value != null && newValue != null) {
      param.value.easing = curveToEasing({ type: easingType.value, power: newValue });
      document.body.focus();
      emit('update');
    }
  },
});
</script>
