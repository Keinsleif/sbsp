<template>
  <v-text-field
    v-model="innerVolume"
    suffix="dB"
    density="compact"
    variant="outlined"
    hide-details
    width="100px"
    autocomplete="off"
    @blur="saveValue"
    @keydown.enter="$event.target.blur()"
    @keydown.esc="
      innerVolume = validateVolume(volume);
      $event.target.blur();
    "
    @keydown.stop
  ></v-text-field>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue';

const volume = defineModel<number>();
const emit = defineEmits(['update']);

const validateVolume = (src: number | undefined): string => {
  if (src == null) {
    return '0.00';
  }
  if (src > 10) {
    return '10.00';
  } else if (src <= -60) {
    return '-∞';
  } else {
    return src.toFixed(2);
  }
};

const innerVolume = ref(validateVolume(volume.value));

watch(volume, () => {
  innerVolume.value = validateVolume(volume.value);
});

const saveValue = () => {
  const newVolume = Number(innerVolume.value);
  if (isNaN(newVolume)) {
    innerVolume.value = validateVolume(volume.value);
    return;
  }
  volume.value = newVolume;
  emit('update');
};
</script>
