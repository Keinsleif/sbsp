<template>
  <v-slider
    hide-details
    v-model="faderPosition"
    :class="props.direction == 'vertical' ? $style['vertical-fader'] : 'mb-2'"
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
      <volume-input
        v-show="!props.hideInput"
        v-model="volume"
        @mousedown.stop
        @mouseup.stop
        @dblclick.stop
      ></volume-input>
    </template>
  </v-slider>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import VolumeInput from './VolumeInput.vue';

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

const props = withDefaults(
  defineProps<{
    label?: string;
    direction?: 'horizontal' | 'vertical';
    hideInput?: boolean;
    thumbAmount?: 'full' | 'decreased' | 'baseOnly';
  }>(),
  {
    direction: 'horizontal',
    hideInput: false,
    thumbAmount: 'full',
  },
);

const volume = defineModel<number>({ default: 0 });

const faderPosition = computed({
  get() {
    return decibelsToFader(volume.value);
  },
  set(newValue) {
    volume.value = faderToDecibels(newValue);
  },
});

const tickLabels = computed(() => {
  if (props.thumbAmount == 'decreased') {
    return {
      10: '10',
      0: '0',
      '-10': '-10',
      '-30': '-60',
    } as Record<number, string>;
  } else if (props.thumbAmount == 'baseOnly') {
    return {
      0: '0',
    } as Record<number, string>;
  } else {
    return {
      10: '10',
      5: '5',
      0: '0',
      '-5': '-5',
      '-10': '-10',
      '-15': '-20',
      '-20': '-30',
      '-25': '-40',
      '-30': '-60',
    } as Record<number, string>;
  }
});
</script>

<style lang="css" module>
.vertical-fader label {
  margin-left: auto;
  margin-right: auto;
  text-align: center;
}
</style>
