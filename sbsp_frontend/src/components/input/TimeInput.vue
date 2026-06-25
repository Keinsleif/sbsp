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
    max?: number | null;
    defaultValue?: number;
    label?: string;
  }>(),
  {
    max: null,
    multiply: 1,
    acceptMinus: false,
    defaultValue: 0,
  },
);

const inputId = useId();
const formattedValue = ref(
  secondsToFormat(seconds.value != null ? seconds.value * props.multiply : null),
);

watch([seconds, () => props.multiply], () => {
  formattedValue.value = secondsToFormat(
    seconds.value != null ? seconds.value * props.multiply : null,
  );
});

const save = () => {
  let innerValue: number;
  if (formattedValue.value.trim() === '') {
    innerValue = props.defaultValue;
  } else {
    innerValue = formatToSeconds(formattedValue.value, props.acceptMinus) / props.multiply;
  }
  if (props.max != null && innerValue > props.max) {
    innerValue = props.max;
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
      formattedValue.value = secondsToFormat(
        seconds.value != null ? seconds.value * props.multiply : null,
      ); // reset
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
      class="w-full h-full"
      variant="outlined"
      autocomplete="off"
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
