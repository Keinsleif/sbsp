<template>
  <v-text-field
    v-if="props.textType == 'single'"
    v-bind="$attrs"
    v-model="innerText"
    :hide-details="!props.showDetails"
    persistent-placeholder
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
    @keydown.stop
  />
  <v-textarea
    v-else-if="props.textType == 'area'"
    v-bind="$attrs"
    v-model="innerText"
    :hide-details="!props.showDetails"
    persistent-placeholder
    variant="outlined"
    density="compact"
    autocomplete="off"
    rows="1"
    auto-grow
    no-resize
    @blur="save"
    @keydown.esc="
      reset();
      $event.target.blur();
    "
    @keydown.tab.prevent="onTabInput"
    @keydown.stop
  />
</template>

<script setup lang="ts">
import { nextTick, ref, watch } from 'vue';

const text = defineModel<string | null>({ default: '' });
const props = withDefaults(
  defineProps<{
    textType?: 'single' | 'area';
    alignInput?: 'left' | 'center' | 'right';
    showDetails?: boolean;
  }>(),
  {
    textType: 'single',
    alignInput: 'center',
    showDetails: false,
  },
);
const emit = defineEmits(['update']);

const innerText = ref(text.value ?? '');

watch(text, () => {
  innerText.value = text.value ?? '';
});

const save = () => {
  if (text.value != innerText.value) {
    text.value = innerText.value;
    emit('update');
  }
};

const reset = () => {
  innerText.value = text.value != null ? text.value : '';
};

const onTabInput = (event: KeyboardEvent) => {
  event.preventDefault();
  const textarea = event.target as HTMLTextAreaElement;
  const start = textarea.selectionStart;
  const end = textarea.selectionEnd;

  const value = innerText.value;
  innerText.value = value.substring(0, start) + '\t' + value.substring(end);

  nextTick(() => {
    textarea.selectionStart = textarea.selectionEnd = start + 1;
  });
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
