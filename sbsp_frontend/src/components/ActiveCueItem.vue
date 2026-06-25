<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { storeToRefs } from 'pinia';
import { useShowModel } from '../stores/showModel';
import type { ActiveCue } from '../types/ActiveCue';
import { computed, useTemplateRef } from 'vue';
import { buildCueName, secondsToFormat } from '../utils';
import { mdiRepeat } from '@mdi/js';
import { usePosition } from '../composables/usePosition';
import PathIcon from './display/PathIcon.vue';
import ProgressSpinnerWrapper from './wrapper/ProgressSpinnerWrapper.vue';
import ProgressBar from 'primevue/progressbar';

const props = defineProps<{
  activeCue: ActiveCue;
}>();

const showModel = useShowModel();
const { getCueById } = storeToRefs(showModel);

const title = computed(() => {
  const activeCue = getCueById.value(props.activeCue.cueId);
  let result = '';
  if (activeCue == null) return result;
  if (activeCue.number.trim() !== '') {
    result = activeCue.number + '・';
  }
  if (activeCue.name != null) {
    result = result + activeCue.name;
  } else {
    result = result + buildCueName(activeCue);
  }
  return result;
});

const elapsedRef = useTemplateRef('elapsed');
const remainRef = useTemplateRef('remain');

usePosition((pos) => {
  if (elapsedRef.value == null || remainRef.value == null) return;
  const position = pos[props.activeCue.cueId];
  if (position == null) return;
  if (props.activeCue.status.startsWith('pre')) {
    elapsedRef.value.textContent = '-' + secondsToFormat(props.activeCue.duration - position);
    remainRef.value.textContent = '00:00.00';
  } else {
    elapsedRef.value.textContent = secondsToFormat(position);
    remainRef.value.textContent = '-' + secondsToFormat(props.activeCue.duration - position);
  }
});
</script>

<template>
  <div class="flex flex-col">
    <div class="flex flex-row items-center justify-between pl-3 pr-3 pt-2">
      <span>{{ title }}</span>
      <path-icon
        v-show="activeCue.status != 'stopping'"
        :icon="
          activeCue.params.type == 'audio' && activeCue.params.repeating === true ? mdiRepeat : null
        "
      />
      <progress-spinner-wrapper
        class="mx-0"
        v-show="activeCue.status == 'stopping'"
        size="21px"
      />
    </div>
    <div class="p-0 flex flex-row justify-between">
      <div
        ref="elapsed"
        class="px-3 py-2"
      />
      <div
        ref="remain"
        class="px-3 py-2"
      />
    </div>
    <progress-bar
      :show-value="false"
      :value="
        activeCue != null && activeCue.duration != 0
          ? Math.ceil((activeCue.position * 100) / activeCue.duration)
          : 0
      "
      style="transition: width 0.1s linear"
      class="h-4 rounded-none"
      :pt="{
        value: {
          style: {
            backgroundColor:
              activeCue.status === 'paused' || activeCue.status === 'stopping'
                ? 'var(--p-orange-500)'
                : 'var(--p-primary-color)',
          },
        },
      }"
    /><!-- use throttle value for model-value -->
  </div>
</template>
