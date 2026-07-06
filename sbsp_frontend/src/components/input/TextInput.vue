<script setup lang="ts">
import FloatLabel from 'primevue/floatlabel';
import InputText from 'primevue/inputtext';
import Message from 'primevue/message';
import { ref, useId, watch } from 'vue';

const model = defineModel<string | null>();
const emit = defineEmits(['update']);
const props = withDefaults(
  defineProps<{
    label?: string;
    placeholder?: string;
    help?: string;
    acceptNull?: boolean;
    disabled?: boolean;
    textAlign?: 'left' | 'center' | 'right';
  }>(),
  {
    acceptNull: false,
    disabled: false,
    textAlign: 'left',
  },
);

const inputId = useId();
const innerText = ref('');

watch(
  model,
  () => {
    innerText.value = model.value ?? '';
  },
  { immediate: true },
);

const save = async () => {
  const origModelString = model.value ?? '';

  if (props.disabled) {
    innerText.value = origModelString;
    return;
  }
  const newText = innerText.value.trim();
  if (newText === '') {
    if (props.acceptNull) {
      // update model by null if acceptNull == true
      // innerText also updated by watcher
      model.value = null;
      emit('update');
      return;
    } else {
      // reset if acceptNull == false and innerText == ''
      innerText.value = origModelString;
      return;
    }
  }

  innerText.value = newText;
  if ((model.value ?? '') !== newText) {
    model.value = newText;
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
      innerText.value = model.value ?? ''; // reset
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
        class="h-full w-full"
        :style="{
          textAlign: props.textAlign,
        }"
        :id="inputId"
        autocomplete="off"
        :placeholder="props.placeholder"
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
    <Message
      v-if="props.help != null"
      size="small"
      severity="secondary"
      variant="simple"
      >{{ props.help }}</Message
    >
  </div>
</template>
