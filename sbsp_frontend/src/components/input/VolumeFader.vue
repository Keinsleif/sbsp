<template>
  <v-slider
    hide-details
    v-model="faderPosition"
    :class="props.direction == 'vertical' ? $style['vertical-fader'] : ''"
    thumb-label
    show-ticks="always"
    step="0.05"
    min="-30"
    max="10"
    :label="props.label"
    :ticks="tickLabels"
    :direction="props.direction"
    @dblclick="faderPosition = 0"
    @keydown.stop
  >
    <template v-slot:thumb-label="{ modelValue }">
      {{ faderToDecibels(modelValue) == -60 ? '-∞dB' : faderToDecibels(modelValue).toFixed(2) + 'dB' }}
    </template>
    <template v-slot:append>
      <v-number-input
        v-model="volume"
        :min="-60"
        :max="10"
        suffix="dB"
        density="compact"
        :precision="2"
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

const faderToDecibels = (fader: number): number => {
  if (fader > -10) {
    return fader;
  } else if (fader > -25) {
    return 2 * (fader + 10) - 10;
  } else {
    return 4 * (fader + 25) - 40;
  }
};

const decibelsToFader = (decibels: number): number => {
  if (decibels > -10) {
    return decibels;
  } else if (decibels > -40) {
    return (decibels + 10) / 2 - 10;
  } else {
    return (decibels + 40) / 4 - 25;
  }
};

const props = defineProps<{
  label?: string;
  direction?: 'horizontal' | 'vertical';
}>();

const volume = defineModel<number>({ default: 0 });

const faderPosition = computed({
  get() {
    return decibelsToFader(volume.value);
  },
  set(newValue) {
    volume.value = faderToDecibels(newValue);
  },
});

const tickLabels = {
  10: '10',
  5: '5',
  0: '0',
  '-5': '-5',
  '-10': '-10',
  '-15': '-20',
  '-20': '-30',
  '-25': '-40',
  '-30': '-60',
};
</script>

<style lang="css" module>
.vertical-fader label {
  margin-left: auto;
  margin-right: auto;
  text-align: center;
}
</style>
