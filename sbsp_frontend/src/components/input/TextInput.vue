<script setup lang="ts">
import FloatLabel from 'primevue/floatlabel';
import InputText from 'primevue/inputtext';
import { ref, useId, watch } from 'vue';

const model = defineModel<string | null>();
const emit = defineEmits(['update']);
const props = defineProps<{
  label?: string;
}>();

const inputId = useId();
const innerText = ref(model.value ?? '');

watch(model, () => {
  innerText.value = model.value ?? '';
});

const save = () => {
  if (model.value !== innerText.value) {
    model.value = innerText.value;
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
      innerText.value = model.value != null ? model.value : ''; // reset
      e.target.blur();
      break;
  }
};
</script>

<template>
  <FloatLabel variant="on">
    <InputText
      v-model="innerText"
      v-bind="$attrs"
      class="w-full h-full"
      :id="inputId"
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
