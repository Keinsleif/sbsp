<template>
  <v-text-field
    hide-details
    persistent-placeholder
    v-model="formattedValue"
    variant="outlined"
    density="compact"
    :class="$style['centered-input']"
    autocomplete="off"
    @blur="save"
    @keydown.enter="$event.target.blur()"
    @keydown.esc="
      reset();
      $event.target.blur();
    "
    @keydown.stop
  ></v-text-field>
</template>

<script setup lang="ts">
  import { ref, watch } from 'vue';
  import { formatToSeconds, secondsToFormat } from '../../utils';

  const seconds = defineModel<number | null>({ default: null });
  const props = withDefaults(
    defineProps<{
      acceptMinus?: boolean;
      max?: number | null;
    }>(),
    {
      acceptMinus: false,
    },
  );
  const emit = defineEmits(['update']);

  const formattedValue = ref(secondsToFormat(seconds.value));

  watch(seconds, () => {
    formattedValue.value = secondsToFormat(seconds.value);
  });

  const save = () => {
    let innerValue = formatToSeconds(formattedValue.value, props.acceptMinus);
    if (props.max != null && innerValue > props.max) {
      innerValue = props.max;
    }
    seconds.value = innerValue;
    if (seconds.value != innerValue) {
      emit('update');
    }
  };

  const reset = () => {
    formattedValue.value = secondsToFormat(seconds.value);
  };
</script>

<style lang="css" module>
  .centered-input input {
    text-align: center;
  }
</style>
