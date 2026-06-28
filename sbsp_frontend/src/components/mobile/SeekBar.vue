<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { computed, ref, useTemplateRef, watch } from 'vue';
import { secondsToFormat } from '../../utils';
import { useShowState } from '../../stores/showState';
import { useApi } from '../../api';
import Slider from 'primevue/slider';
import { usePosition } from '@/composables/usePosition';

const props = defineProps<{
  targetId: string | null;
}>();

const api = useApi();
const showState = useShowState();

const position = ref(0);
const sliderChanging = ref(false);

const activeTargetCue = computed(() => {
  if (props.targetId == null) return null;
  const activeCue = showState.activeCues[props.targetId];
  if (activeCue == null) return null;
  return activeCue;
});

const elapsedRef = useTemplateRef('elapsed');
const remainRef = useTemplateRef('remain');
usePosition((pos) => {
  if (elapsedRef.value == null || remainRef.value == null || props.targetId == null) return;
  const cuePos = pos[props.targetId];
  const activeCue = activeTargetCue.value;
  if (cuePos == null || activeCue == null) {
    if (elapsedRef.value.textContent !== '--:--.--') {
      elapsedRef.value.textContent = '--:--.--';
    }
    if (remainRef.value.textContent !== '--:--.--') {
      remainRef.value.textContent = '--:--.--';
    }
  } else {
    elapsedRef.value.textContent = secondsToFormat(cuePos);
    remainRef.value.textContent = secondsToFormat(activeCue.duration - cuePos);
  }
});

// use throttle value for v-model value
watch(() => activeTargetCue.value?.position, (newposition) => {
  if (!sliderChanging.value) {
    position.value = newposition || 0;
  }
});

const onpointerup = () => {
  if (sliderChanging.value && props.targetId != null) {
    sliderChanging.value = false;
    api.sendSeekTo(props.targetId, position.value);
  }
};
</script>

<template>
  <div class="flex flex-col">
    <Slider
      v-model="position"
      class="grow-0"
      :readonly="activeTargetCue==null"
      :severity="activeTargetCue?.status.startsWith('pre') ? 'warn' : 'primary'"
      :min="0"
      :max="activeTargetCue?.duration || 1"
      @pointerdown="sliderChanging = true"
      @pointerup="onpointerup"
    />
    <div class="flex flex-row mt-2">
      <div ref="elapsed" class="px-1 ml-0 mr-auto"></div>
      <div ref="remain" class="px-1 ml-auto mr-0"></div>
    </div>
  </div>
</template>
