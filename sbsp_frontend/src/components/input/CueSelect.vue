<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { computed } from 'vue';
import { useShowModel } from '../../stores/showModel';
import type { Cue } from '../../types/Cue';
import { buildCueName } from '../../utils';
import Select from 'primevue/select';
import FloatLabel from 'primevue/floatlabel';

const showModel = useShowModel();

const selectedId = defineModel<string | null>();

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
    list.unshift({ value: null, name: props.nullText });
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

const onHide = () => {
  if (document.activeElement instanceof HTMLElement) {
    document.activeElement.blur();
  }
};
</script>

<template>
  <float-label
    variant="on"
    @keydown.stop
  >
    <Select
      v-bind="$attrs"
      v-model="selectedId"
      :options="cueList"
      option-value="value"
      option-label="name"
      autocomplete="off"
      :pt="{
        root: () => {
          return {
            class: 'w-full p-inputwrapper-filled',
            style: 'background-color: var(--p-inputtext-background);',
          };
        },
      }"
      @update:model-value="emit('update')"
      @hide="onHide"
    >
      <template #value="innerProps">
        {{ cueList.find((opt) => opt.value === (innerProps.value || null))?.name || ' ' }}
      </template>
    </Select>
    <label>{{ props.label }}</label>
    <!--label cannot be attachable. Cue select is not generic input form.-->
  </float-label>
</template>
