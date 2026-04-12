<template>
  <v-range-slider
    v-model="innerRange"
    hide-details
    min="0"
    :max="props.duration"
    @pointerdown="sliderChanging = true"
    @pointerup="
      if (sliderChanging) {
        sliderChanging = false;
        emit('update');
      }
    "
    @keydown.stop
  >
    <template #prepend>
      <time-input
        v-model="range[0]"
        width="100px"
        :label="t('main.bottomEditor.timeLevels.startTime')"
        @update="emit('update')"
        @pointerdown.stop
      />
    </template>
    <template #append>
      <time-input
        v-model="range[1]"
        width="100px"
        :label="t('main.bottomEditor.timeLevels.endTime')"
        @update="emit('update')"
        @pointerdown.stop
      />
    </template>
  </v-range-slider>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue';
import TimeInput from './TimeInput.vue';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();

const sliderChanging = ref(false);

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
    range.value[0] = newValue[0] != 0 ? newValue[0]! : null;
    range.value[1] = newValue[1] != props.duration ? newValue[1]! : null;
  },
});
</script>
