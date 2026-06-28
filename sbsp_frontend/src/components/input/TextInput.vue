<script setup lang="ts">
import FloatLabel from 'primevue/floatlabel';
import InputText from 'primevue/inputtext';
import Message from 'primevue/message';
import { ref, useId, watch } from 'vue';

const model = defineModel<string | null>();
const emit = defineEmits(['update']);
const props = defineProps<{
  label?: string;
  help?: string;
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
  <div class="flex flex-col gap-1">
    <FloatLabel variant="on">
      <InputText
        v-model="innerText"
        v-bind="$attrs"
        class="h-full w-full"
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
    <Message
      v-if="props.help != null"
      size="small"
      severity="secondary"
      variant="simple"
      >{{ props.help }}</Message
    >
  </div>
</template>
