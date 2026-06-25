<script setup lang="ts">
import FloatLabel from 'primevue/floatlabel';
import Textarea from 'primevue/textarea';
import { nextTick, ref, useId, watch } from 'vue';

const text = defineModel<string | null>();
const emit = defineEmits(['update']);
const props = defineProps<{
  label?: string;
}>();

const inputId = useId();
const innerText = ref(text.value ?? '');

watch(text, () => {
  innerText.value = text.value ?? '';
});

const save = () => {
  if (text.value !== innerText.value) {
    text.value = innerText.value;
    emit('update');
  }
};

const onKeydown = (e: KeyboardEvent) => {
  if (!(e.target instanceof HTMLElement)) return;
  switch (e.key) {
    case 'Escape':
      innerText.value = text.value != null ? text.value : ''; // reset
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
      class="w-full h-full"
      :id="inputId"
      autocomplete="off"
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
