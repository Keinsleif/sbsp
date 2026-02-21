<template>
  <div class="pa-2 border-md rounded position-relative">
    <span class="position-absolute text-caption text-medium-emphasis" style="top: 4px; left: 8px">
      {{ props.label }}
    </span>
    <v-sheet class="d-flex flex-row align-end ga-2">
      <v-checkbox
        hide-details
        v-show="!props.disableToggle"
        :disabled="props.disabled || props.disableToggle"
        v-model="fadeEnabled"
        @update:model-value="saveValues"
      ></v-checkbox>
      <time-input
        v-model="duration"
        :disabled="!fadeEnabled || props.disabled"
        class="flex-grow-0"
        :label="t('main.duration')"
        width="100px"
        @update="saveValues"
      ></time-input>
      <v-select
        hide-details
        persistent-placeholder
        v-model="easingType"
        :label="t('main.bottomEditor.input.curve')"
        class="flex-grow-0"
        width="160px"
        :items="[
          { value: 'linear', name: t('main.bottomEditor.input.linear') },
          { value: 'inPow', name: t('main.bottomEditor.input.easeIn') },
          { value: 'outPow', name: t('main.bottomEditor.input.easeOut') },
          { value: 'inOutPow', name: t('main.bottomEditor.input.easeInOut') },
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
        max-width="150px"
        :label="t('main.bottomEditor.input.intensity')"
        density="compact"
        variant="outlined"
        autocomplete="off"
        @update:model-value="saveValues"
        @keydown.stop
      ></v-number-input>
      <curve-viewer
        :disabled="!fadeEnabled || props.disabled"
        class="border-md"
        width="68px"
        :type="props.condition != 'out' ? 'in' : 'out'"
        :curve="easingType"
        :power="easingPower"
      ></curve-viewer>
      <curve-viewer
        :disabled="!fadeEnabled || props.disabled"
        v-if="props.condition == 'both'"
        class="border-md"
        width="68px"
        type="out"
        :curve="easingType"
        :power="easingPower"
      ></curve-viewer>
    </v-sheet>
  </div>
</template>

<script setup lang="ts">
  import { ref, watch } from 'vue';
  import { curveToEasing, easingToCurve } from '../../utils';
  import CurveViewer from './CurveViewer.vue';
  import TimeInput from './TimeInput.vue';
  import { useI18n } from 'vue-i18n';
  import type { FadeParam } from '../../types/FadeParam';

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
  const easingPower = ref<number | null>(param.value != null ? easingToCurve(param.value.easing).power : null);

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
          easing: { type: 'inOutPowi', intensity: 2 },
        };
        duration.value = 3;
        easingType.value = 'inOutPow';
        easingPower.value = 2;
      } else {
        if (duration.value != null) {
          param.value.duration = duration.value;
        }
        if (easingType.value == 'linear') {
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
