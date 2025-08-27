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
  ></v-text-field>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue';
import { formatToSeconds, secondsToFormat } from '../../utils';

const seconds = defineModel<number | null>({ default: null });
const props = withDefaults(
  defineProps<{
    acceptMinus?: boolean;
  }>(),
  {
    acceptMinus: false,
  },
);
const emit = defineEmits(['update']);

const formattedValue = ref(seconds.value != null ? secondsToFormat(seconds.value) : '--:--.--');

watch(seconds, () => {
  formattedValue.value = seconds.value != null ? secondsToFormat(seconds.value) : '--:--.--';
});

const save = () => {
  const innerValue = formatToSeconds(formattedValue.value, props.acceptMinus);
  seconds.value = innerValue;
  if (seconds.value != innerValue) {
    emit('update');
  }
};

const reset = () => {
  formattedValue.value = seconds.value != null ? secondsToFormat(seconds.value) : '--:--.--';
};
</script>

<style lang="css" module>
.centered-input input {
  text-align: center;
}
</style>
