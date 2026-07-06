<script setup lang="ts">
import { formatToSeconds, secondsToFormat } from '@/utils';
import FloatLabel from 'primevue/floatlabel';
import InputText from 'primevue/inputtext';
import { ref, useId, watch } from 'vue';

const seconds = defineModel<number | null>();
const emit = defineEmits(['update']);
const props = withDefaults(
  defineProps<{
    acceptMinus?: boolean;
    multiply?: number;
    max?: number;
    min?: number;
    defaultValue?: number;
    label?: string;
    disabled?: boolean;
  }>(),
  {
    multiply: 1,
    acceptMinus: false,
    defaultValue: 0,
  },
);

const model2text = (value: number | null | undefined) => {
  return secondsToFormat(value != null ? value * props.multiply : null);
};

const inputId = useId();
const formattedValue = ref('');

watch(
  [seconds, () => props.multiply],
  ([newSeconds]) => {
    formattedValue.value = model2text(newSeconds);
  },
  { immediate: true },
);

const save = () => {
  if (props.disabled) return;
  const origModelString = model2text(seconds.value);
  if (formattedValue.value.trim() === origModelString) return;
  let innerValue: number;
  if (formattedValue.value.trim() === '') {
    innerValue = props.defaultValue;
  } else {
    innerValue = formatToSeconds(formattedValue.value, props.acceptMinus) / props.multiply;
  }
  if (props.max != null && innerValue > props.max) {
    innerValue = props.max;
  } else if (props.min != null && innerValue < props.min) {
    innerValue = props.min;
  }
  if (seconds.value !== innerValue) {
    seconds.value = innerValue;
    emit('update');
  }
};

const onKeydown = (e: KeyboardEvent) => {
  if (!(e.target instanceof HTMLElement)) return;
  switch (e.key) {
    case 'Enter':
      e.target.blur();
      break;
    case 'Escape':
      formattedValue.value = model2text(seconds.value); // reset
      e.target.blur();
      break;
  }
};
</script>

<template>
  <FloatLabel variant="on">
    <InputText
      v-model="formattedValue"
      v-bind="$attrs"
      :id="inputId"
      class="h-full w-full"
      variant="outlined"
      autocomplete="off"
      :disabled="props.disabled"
      :pt="{
        root: () => {
          return {
            style: 'background-color: var(--p-inputtext-background);',
          };
        },
      }"
      @blur="save"
      @keydown.stop="onKeydown"
    />
    <label :for="inputId">{{ props.label || '' }}</label>
  </FloatLabel>
</template>
