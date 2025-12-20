<template>
  <v-range-slider
    hide-details
    v-model="innerRange"
    min="0"
    :max="props.duration"
    @update:model-value="emit('update')"
    @keydown.stop
  >
    <template v-slot:prepend>
      <time-input
        v-model="range[0]"
        width="100px"
        :label="t('main.bottomEditor.timeLevels.startTime')"
        @update="emit('update')"
      ></time-input>
    </template>
    <template v-slot:append>
      <time-input
        v-model="range[1]"
        width="100px"
        :label="t('main.bottomEditor.timeLevels.endTime')"
        @update="emit('update')"
      ></time-input>
    </template>
  </v-range-slider>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import TimeInput from './TimeInput.vue';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();

const range = defineModel<[number | null, number | null]>({ required: true });
const props = withDefaults(
  defineProps<{
    duration?: number;
  }>(),
  {
    duration: 0,
  },
);
const emit = defineEmits(['update']);

const innerRange = computed({
  get() {
    return [range.value[0] != null ? range.value[0] : 0, range.value[1] != null ? range.value[1] : props.duration];
  },
  set(newValue) {
    range.value[0] = newValue[0] != 0 ? newValue[0] : null;
    range.value[1] = newValue[1] != props.duration ? newValue[1] : null;
  },
});
</script>
