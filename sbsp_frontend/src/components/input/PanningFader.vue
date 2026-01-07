<template>
  <v-slider
    hide-details
    class="mb-2"
    v-model="faderPosition"
    thumb-label
    show-ticks="always"
    step="1"
    min="-64"
    max="64"
    :label="props.label"
    :ticks="tickLabels"
    :direction="props.direction"
    @dblclick="faderPosition = 0"
    @mousedown="sliderChanging = true"
    @mouseup="
      if (sliderChanging) {
        sliderChanging = false;
        emit('update');
      }
    "
    @keydown.stop
  >
    <template v-slot:thumb-label="{ modelValue }">
      {{ thumbLabel(modelValue) }}
    </template>
    <template v-slot:append>
      <v-number-input
        v-show="!props.hideInput"
        v-model="panning"
        :min="-1"
        :max="1"
        :step="1 / 8"
        :prefix="panning < 0 ? 'L' : panning > 0 ? 'R' : 'C'"
        density="compact"
        :precision="3"
        variant="outlined"
        control-variant="hidden"
        hide-details
        width="100px"
        @mousedown.stop
        @dblclick.stop
      ></v-number-input>
    </template>
  </v-slider>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue';

const props = withDefaults(
  defineProps<{
    label?: string;
    direction?: 'horizontal' | 'vertical';
    hideInput?: boolean;
  }>(),
  {
    direction: 'horizontal',
    hideInput: false,
  },
);

const panning = defineModel<number>({ default: 0 });
const emit = defineEmits(['update']);

const sliderChanging = ref(false);

const faderPosition = computed({
  get() {
    return panning.value * 64;
  },
  set(newValue) {
    panning.value = newValue / 64;
  },
});

const thumbLabel = (value: number): string => {
  if (value == 0) {
    return 'Center';
  } else if (value > 0) {
    return 'R' + Math.abs(value);
  } else {
    return 'L' + Math.abs(value);
  }
};

const tickLabels = {
  64: 'R',
  0: 'C',
  '-64': 'L',
};
</script>
