<template>
  <v-select
    hide-details
    persistent-placeholder
    v-model="selectedId"
    :label="props.label"
    :items="showModel.cues.filter(filterCue).map((cue) => ({ value: cue.id, name: buildCueName(cue) }))"
    item-value="value"
    item-title="name"
    variant="outlined"
    density="compact"
    autocomplete="off"
    @update:modelValue="emit('update')"
    @keydown.stop
  ></v-select>
</template>

<script setup lang="ts">
import { useShowModel } from '../../stores/showmodel';
import type { Cue } from '../../types/Cue';
import { buildCueName } from '../../utils';

const showModel = useShowModel();

const selectedId = defineModel<string | null>();
const props = withDefaults(
  defineProps<{
    label?: string;
    cueType: 'audio' | 'wait' | 'all';
  }>(),
  {
    label: '',
    cueType: 'all',
  },
);
const emit = defineEmits(['update']);

const filterCue = (cue: Cue): boolean => {
  if (props.cueType == 'all') {
    return true;
  } else {
    return cue.params.type == props.cueType;
  }
};
</script>
