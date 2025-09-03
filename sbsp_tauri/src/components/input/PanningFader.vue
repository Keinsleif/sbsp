<template>
  <v-slider
    hide-details
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
    @keydown.stop
  >
    <template v-slot:thumb-label="{ modelValue }">
      {{ thumbLabel(modelValue) }}
    </template>
    <template v-slot:append>
      <v-number-input
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
        @dblclick.stop
      ></v-number-input>
    </template>
  </v-slider>
</template>

<script setup lang="ts">
import { computed } from 'vue';

const props = defineProps<{
  label?: string;
  direction?: 'horizontal' | 'vertical';
}>();

const panning = defineModel<number>({ default: 0 });

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
