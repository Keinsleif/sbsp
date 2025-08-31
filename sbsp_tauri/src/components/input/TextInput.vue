<template>
  <v-text-field
    v-if="props.type == 'single'"
    v-bind="$attrs"
    hide-details
    persistent-placeholder
    v-model="innerText"
    variant="outlined"
    density="compact"
    autocomplete="off"
    :class="
      props.alignInput == 'left'
        ? $style['left-input']
        : props.alignInput == 'center'
          ? $style['center-input']
          : $style['right-input']
    "
    @blur="save"
    @keydown.enter="$event.target.blur()"
    @keydown.esc="
      reset();
      $event.target.blur();
    "
  ></v-text-field>
  <v-textarea
    v-if="props.type == 'area'"
    v-bind="$attrs"
    hide-details
    persistent-placeholder
    v-model="innerText"
    variant="outlined"
    density="compact"
    autocomplete="off"
    no-resize
    @blur="save"
    @keydown.esc="
      reset();
      $event.target.blur();
    "
    @keydown.tab.prevent="onTabInput"
  ></v-textarea>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue';

const text = defineModel<string | null>({ default: '' });
const props = withDefaults(
  defineProps<{
    type?: 'single' | 'area';
    alignInput?: 'left' | 'center' | 'right';
  }>(),
  {
    type: 'single',
    alignInput: 'center',
  },
);
const emit = defineEmits(['update']);

const innerText = ref(text.value != null ? text.value : '');

watch(text, () => {
  innerText.value = text.value != null ? text.value : '';
});

const save = () => {
  if (text.value != innerText.value) {
    emit('update');
  }
  text.value = innerText.value;
};

const reset = () => {
  innerText.value = text.value != null ? text.value : '';
};

const onTabInput = (event: KeyboardEvent) => {
  event.preventDefault();
  const textarea = event.target as HTMLTextAreaElement;
  const start = textarea.selectionStart;
  const end = textarea.selectionEnd;

  textarea.setRangeText('\t', start, end, 'end');
  const inputEvent = new InputEvent('input', {
    bubbles: true,
    cancelable: false,
    inputType: 'insertText',
    data: '\t',
  });
  textarea.dispatchEvent(inputEvent);
};
</script>

<style lang="css" module>
.center-input input {
  text-align: center;
}
.left-input input {
  text-align: left;
}
.right-input input {
  text-align: right;
}
</style>
