<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { computed } from 'vue';
import { useShowModel } from '../../stores/showModel';
import type { Cue } from '../../types/Cue';
import { buildCueName } from '../../utils';
import Select from 'primevue/select';
import FloatLabel from 'primevue/floatlabel';
import { NIL } from 'uuid';

const showModel = useShowModel();

const selectedId = defineModel<string | null>();

const innerId = computed({
  get() {
    return selectedId.value || NIL;
  },
  set(value) {
    selectedId.value = value === NIL ? null : value;
  },
});

const props = withDefaults(
  defineProps<{
    label?: string;
    cueType?: ('audio' | 'wait' | 'stop' | 'start' | 'load' | 'pause' | 'fade' | 'group')[] | 'all';
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
  const list: { value: string | null; name: string }[] = showModel.flatCueList
    .filter((item) => filterCue(item.cue))
    .map((item) => ({ value: item.cue.id, name: buildCueName(item.cue) }));
  if (props.nullText != null) {
    list.unshift({ value: NIL, name: props.nullText });
  }
  return list;
});

const filterCue = (cue: Cue): boolean => {
  if (cue.id === props.exclude) {
    return false;
  }
  if (props.cueType == null || props.cueType === 'all') {
    return true;
  } else {
    return props.cueType.includes(cue.params.type);
  }
};
</script>

<template>
  <float-label variant="on">
    <Select
      v-bind="$attrs"
      v-model="innerId"
      :options="cueList"
      option-value="value"
      option-label="name"
      autocomplete="off"
      :pt="{
        root: () => {
          return {
            style: 'background-color: var(--p-inputtext-background);',
          };
        },
      }"
      @update:model-value="emit('update')"
      @keydown.stop
    />
    <label>{{ props.label }}</label>
  </float-label>
</template>
