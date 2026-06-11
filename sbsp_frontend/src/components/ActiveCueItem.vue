<template>
  <div
    class="d-flex flex-column border"
  >
    <div class="d-flex flex-row align-center justify-space-between pl-3 pr-3 pt-2">
      <span>{{ title }}</span>
      <v-icon
        v-show="activeCue.status != 'stopping'"
        :icon="activeCue.params.type == 'audio' && activeCue.params.repeating === true ? mdiRepeat : undefined"
      />
      <v-progress-circular
        v-show="activeCue.status == 'stopping'"
        indeterminate="disable-shrink"
        size="21"
      />
    </div>
    <div class="pa-0 d-flex flex-row justify-space-between">
      <div class="px-3 py-2" ref="elapsed"></div>
      <div class="px-3 py-2" ref="remain"></div>
    </div>
    <v-progress-linear
      :color="activeCue.status == 'paused' || activeCue.status == 'stopping' ? 'warning' : 'primary'"
      style="transition: width 0.1s linear;"
      :model-value="activeCue != null && activeCue.duration != 0 ? (activeCue.position * 100) / activeCue.duration : 0"
      height="16"
    /> <!-- use throttle value for model-value -->
  </div>
</template>

<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { storeToRefs } from 'pinia';
import { useShowModel } from '../stores/showmodel';
import type { ActiveCue } from '../types/ActiveCue';
import { computed, useTemplateRef } from 'vue';
import { buildCueName, secondsToFormat } from '../utils';
import { mdiRepeat } from '@mdi/js';
import { usePosition } from '../composables/usePosition';

const props = defineProps<{
  activeCue: ActiveCue;
}>();

const showModel = useShowModel();
const { getCueById } = storeToRefs(showModel);

const title = computed(() => {
  const activeCue = getCueById.value(props.activeCue.cueId);
  if (activeCue == null) return;
  let result = '';
  if (activeCue.number.trim() != '') {
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
