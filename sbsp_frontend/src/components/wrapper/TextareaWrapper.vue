<script setup lang="ts">
import FloatLabel from 'primevue/floatlabel';
import Textarea from 'primevue/textarea';
import { nextTick, ref, useId, watch } from 'vue';

const text = defineModel<string | null>();
const emit = defineEmits(['update']);
const props = defineProps<{
  label?: string;
  disabled?: boolean;
}>();

const inputId = useId();
const innerText = ref('');

watch(
  text,
  () => {
    innerText.value = text.value ?? '';
  },
  { immediate: true },
);

const save = () => {
  if (props.disabled) {
    innerText.value = text.value ?? ''; // reset
    return;
  }
  if ((text.value ?? '') !== innerText.value) {
    text.value = innerText.value.trim() === '' ? null : innerText.value;
    emit('update');
  }
};

const onKeydown = (e: KeyboardEvent) => {
  if (!(e.target instanceof HTMLElement)) return;
  switch (e.key) {
    case 'Escape':
      innerText.value = text.value ?? ''; // reset
      e.target.blur();
      break;
    case 'Tab':
      if (e.target instanceof HTMLTextAreaElement) {
        e.preventDefault();
        const textarea = e.target;
        const start = textarea.selectionStart;
        const end = textarea.selectionEnd;

        const value = innerText.value;
        innerText.value = value.substring(0, start) + '\t' + value.substring(end);

        nextTick(() => {
          textarea.selectionStart = textarea.selectionEnd = start + 1;
        });
        break;
      }
  }
};
</script>

<template>
  <FloatLabel variant="on">
    <Textarea
      v-model="innerText"
      v-bind="$attrs"
      class="h-full w-full"
      :id="inputId"
      autocomplete="off"
      :disabled="props.disabled"
      :pt="{
        root: () => {
          return {
            style: 'background-color: var(--p-textarea-background);',
          };
        },
      }"
      @blur="save"
      @keydown.stop="onKeydown"
    />
    <label :for="inputId">{{ props.label || '' }}</label>
  </FloatLabel>
</template>
