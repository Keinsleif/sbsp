<template>
  <v-slider
    v-model="faderPosition"
    hide-details
    :class="props.direction == 'vertical' ? $style['vertical-fader'] : 'mb-2'"
    thumb-label
    show-ticks="always"
    step="0.05"
    min="-30"
    max="10"
    :label="props.label"
    :ticks="tickLabels"
    :direction="props.direction"
    @dblclick="
      faderPosition = 0;
      onPointerUp.clear();
      emit('update');
    "
    @keydown.stop
    @pointerdown="sliderChanging = true"
    @pointerup="onPointerUp"
  >
    <template #thumb-label="{ modelValue }">
      {{ faderToDecibels(modelValue) == -60 ? '-∞dB' : faderToDecibels(modelValue).toFixed(2) + 'dB' }}
    </template>
    <template #append>
      <volume-input
        v-show="!props.hideInput"
        v-model="volume"
        @pointerdown.stop
        @dblclick.stop
        @update="emit('update')"
      />
    </template>
  </v-slider>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue';
import VolumeInput from './VolumeInput.vue';
import { debounce, decibelsToFader, faderToDecibels } from '../../utils';

const props = withDefaults(
  defineProps<{
    label?: string;
    direction?: 'horizontal' | 'vertical';
    hideInput?: boolean;
    thumbAmount?: 'full' | 'decreased' | 'baseOnly';
  }>(),
  {
    label: 'Volume',
    direction: 'horizontal',
    hideInput: false,
    thumbAmount: 'full',
  },
);

const volume = defineModel<number>({ default: 0 });
const emit = defineEmits(['update']);

const sliderChanging = ref(false);

const faderPosition = computed({
  get() {
    return decibelsToFader(volume.value);
  },
  set(newValue) {
    volume.value = faderToDecibels(newValue);
  },
});

const onPointerUp = debounce(() => {
  if (sliderChanging.value) {
    sliderChanging.value = false;
    emit('update');
  }
}, 300);

const tickLabels = computed(() => {
  if (props.thumbAmount == 'decreased') {
    return {
      '10': '10',
      '0': '0',
      '-10': '-10',
      '-30': '-60',
    } as Record<number, string>;
  } else if (props.thumbAmount == 'baseOnly') {
    return {
      0: '0',
    } as Record<number, string>;
  } else {
    return {
      '10': '10',
      '5': '5',
      '0': '0',
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
