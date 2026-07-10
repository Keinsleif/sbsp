<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { useTimeoutFn } from '@vueuse/core';
import { onMounted, onUnmounted, ref, useTemplateRef } from 'vue';
import { useLevelMeterListener } from '../../composables/useLevelMeterListener';

const DECAY_PER_SEC = 30;

const props = withDefaults(
  defineProps<{
    kind?: 'master';
    width?: string;
  }>(),
  {
    kind: 'master',
    width: '4px',
  },
);

const leftRef = useTemplateRef('left');
const rightRef = useTemplateRef('right');

const levels = {
  left: -60,
  right: -60,
};

const clipping = ref({
  left: false,
  right: false,
});

const { start: startLeftClipReset, stop: stopLeftClipReset } = useTimeoutFn(() => {
  clipping.value.left = false;
}, 500);
const { start: startRightClipReset, stop: stopRightClipReset } = useTimeoutFn(() => {
  clipping.value.right = false;
}, 500);

let animationFrameId: number;

let lastTime = 0;
const decayLoop = (timestamp: number) => {
  if (!lastTime) lastTime = timestamp;
  const deltaTime = (timestamp - lastTime) / 1000;
  lastTime = timestamp;

  if (levels.left > -60) {
    levels.left = Math.max(-60, levels.left - DECAY_PER_SEC * deltaTime);
  }
  if (levels.right > -60) {
    levels.right = Math.max(-60, levels.right - DECAY_PER_SEC * deltaTime);
  }

  if (levels.left > 0) {
    clipping.value.left = true;
    stopLeftClipReset();
    startLeftClipReset();
  }

  if (levels.right > 0) {
    clipping.value.right = true;
    stopRightClipReset();
    startRightClipReset();
  }

  if (leftRef.value != null && rightRef.value != null) {
    leftRef.value.style.transform = `scaleY(${Math.min(levels.left, 0) / -60})`;
    rightRef.value.style.transform = `scaleY(${Math.min(levels.right, 0) / -60})`;
  }

  animationFrameId = requestAnimationFrame(decayLoop);
};

const toDb = (sample: number) =>
  Number.isFinite(sample) && sample > 0 ? 20 * Math.log10(sample) : -60;

onMounted(() => {
  useLevelMeterListener((message) => {
    levels.left = Math.max(levels.left, toDb(message[0]));
    levels.right = Math.max(levels.right, toDb(message[1]));
  });
  animationFrameId = requestAnimationFrame(decayLoop);
});

onUnmounted(() => {
  cancelAnimationFrame(animationFrameId);
});
</script>

<template>
  <div class="flex flex-col items-center gap-2 p-2">
    <div class="text-center">
      {{ props.kind == 'master' ? 'Master' : '' }}
    </div>
    <div class="flex flex-row h-full">
      <div
        class="relative flex flex-col h-full"
        :style="{ top: '-' + props.width }"
      >
        <div
          class="relative border border-(--p-form-field-border-color)"
          :class="clipping.left ? 'bg-red' : 'bg-surface'"
          style="box-sizing: content-box; top: -2px"
          :style="{ width: props.width, height: props.width }"
        />
        <div
          class="border border-(--p-form-field-border-color) grow"
          :class="$style['meter-bar']"
          :style="{ width: props.width }"
        >
          <div
            ref="left"
            class="relative top-0 left-0 bg-(--p-content-background) h-full origin-top"
            :style="{
              width: props.width,
            }"
          />
        </div>
      </div>
      <div
        class="relative h-full"
        style="font-size: 0.6em; width: 1.8em"
      >
        <div
          class="absolute text-center"
          style="width: 100%; transform: translateY(-50%); top: 0"
        >
          0
        </div>
        <div
          class="absolute text-center"
          style="width: 100%; transform: translateY(-50%); top: 10%"
        >
          6
        </div>
        <div
          class="absolute text-center"
          style="width: 100%; transform: translateY(-50%); top: 20%"
        >
          12
        </div>
        <div
          class="absolute text-center"
          style="width: 100%; transform: translateY(-50%); top: 30%"
        >
          18
        </div>
        <div
          class="absolute text-center"
          style="width: 100%; transform: translateY(-50%); top: 40%"
        >
          24
        </div>
        <div
          class="absolute text-center"
          style="width: 100%; transform: translateY(-50%); top: 50%"
        >
          30
        </div>
        <div
          class="absolute text-center"
          style="width: 100%; transform: translateY(-50%); top: 70%"
        >
          42
        </div>
        <div
          class="absolute text-center"
          style="width: 100%; transform: translateY(-80%); top: 100%"
        >
          60
        </div>
      </div>
      <div
        class="relative flex flex-col h-full"
        :style="{ top: '-' + props.width }"
      >
        <div
          class="relative border border-(--p-form-field-border-color)"
          :class="clipping.right ? 'bg-red' : 'bg-surface'"
          style="box-sizing: content-box; top: -2px"
          :style="{ width: props.width, height: props.width }"
        />
        <div
          class="border border-(--p-form-field-border-color) grow"
          :class="$style['meter-bar']"
          :style="{ width: props.width }"
        >
          <div
            ref="right"
            class="relative top-0 left-0 bg-(--p-content-background) h-full origin-top"
            :style="{ width: props.width }"
          />
        </div>
      </div>
    </div>
  </div>
</template>

<style lang="css" module>
.meter-bar {
  background: linear-gradient(
      to top,
      rgb(52, 211, 103) 70%,
      rgb(251, 191, 36) 70%,
      rgb(251, 191, 36) 95%,
      rgb(255, 0, 0) 95%
    )
    no-repeat;
  box-sizing: content-box;
}
</style>
