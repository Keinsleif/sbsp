<template>
  <v-select
    hide-details
    persistent-placeholder
    v-model="selectedId"
    :label="props.label"
    :items="cueList"
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
import { computed } from 'vue';
import { useShowModel } from '../../stores/showmodel';
import type { Cue } from '../../types/Cue';
import { buildCueName } from '../../utils';

const showModel = useShowModel();

const selectedId = defineModel<string | null>();
const props = withDefaults(
  defineProps<{
    label?: string;
    cueType?: 'audio' | 'wait' | 'all';
    exclude?: string;
    nullText?: string | null;
  }>(),
  {
    label: '',
    cueType: 'all',
    exclude: '',
    nullText: null,
  },
);
const emit = defineEmits(['update']);

const cueList = computed(() => {
  const list: { value: string | null; name: string }[] = showModel.cues
    .filter(filterCue)
    .map((cue) => ({ value: cue.id, name: buildCueName(cue) }));
  if (props.nullText != null) {
    list.unshift({ value: null, name: props.nullText });
  }
  return list;
});

const filterCue = (cue: Cue): boolean => {
  if (cue.id == props.exclude) {
    return false;
  }
  if (props.cueType == 'all') {
    return true;
  } else {
    return cue.params.type == props.cueType;
  }
};
</script>
