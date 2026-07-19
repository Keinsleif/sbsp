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

const props = defineProps<{
  activeCue: ActiveCue;
  isHidden: boolean;
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
const progressRef = useTemplateRef('progress');

usePosition((pos) => {
  if (props.isHidden) return;
  if (elapsedRef.value == null || remainRef.value == null || progressRef.value == null) return;
  const position = pos[props.activeCue.cueId];
  if (position == null) return;
  if (props.activeCue.status.startsWith('pre')) {
    if (props.activeCue.duration > 0) {
      elapsedRef.value.textContent = '-' + secondsToFormat(props.activeCue.duration - position);
    }
    remainRef.value.textContent = '00:00.00';
  } else {
    elapsedRef.value.textContent = secondsToFormat(position);
    if (props.activeCue.duration > 0) {
      remainRef.value.textContent = '-' + secondsToFormat(props.activeCue.duration - position);
    }
  }
  if (props.activeCue.duration > 0) {
    progressRef.value.style.transform = `scaleX(${position / props.activeCue.duration})`;
  }
});
</script>

<template>
  <div class="flex flex-col">
    <div class="flex flex-row items-center justify-between pt-2 pr-3 pl-3">
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
    <div class="flex flex-row justify-between p-0">
      <div
        ref="elapsed"
        class="px-3 py-2"
      />
      <div
        ref="remain"
        class="px-3 py-2"
      />
    </div>
    <div class="h-4 w-full border-y border-(--p-form-field-border-color)">
      <div
        ref="progress"
        class="h-full w-full origin-left"
        :style="{
          backgroundColor:
            activeCue.status === 'paused' || activeCue.status === 'stopping'
              ? 'var(--p-orange-500)'
              : 'var(--p-primary-color)',
        }"
      />
    </div>
  </div>
</template>
