<template>
  <div class="d-flex flex-column align-center pa-2 ga-2">
    <div class="text-center">{{ props.kind == 'master' ? 'Master' : '' }}</div>
    <div class="d-flex flex-row">
      <div class="d-flex flex-column position-relative" :style="{ top: '-' + props.width }">
        <div
          class="border position-relative"
          :class="clipping.left ? 'bg-red' : 'bg-surface'"
          style="box-sizing: content-box; top: -2px"
          :style="{ width: props.width, height: props.width }"
        ></div>
        <div class="border" :class="$style['meter-bar']" :style="{ width: props.width, height: props.height }">
          <div
            class="position-relative top-0 left-0 bg-surface"
            style="transition: height 40ms ease-out"
            :style="{
              width: props.width,
              height: (levels.left * 100) / -60 + '%',
            }"
          ></div>
        </div>
      </div>
      <div class="d-flex flex-column position-relative" style="font-size: 0.6em; width: 1.8em">
        <div class="text-center position-absolute" style="width: 100%; transform: translateY(-50%); top: 0">0</div>
        <div class="text-center position-absolute" style="width: 100%; transform: translateY(-50%); top: 10%">6</div>
        <div class="text-center position-absolute" style="width: 100%; transform: translateY(-50%); top: 20%">12</div>
        <div class="text-center position-absolute" style="width: 100%; transform: translateY(-50%); top: 30%">18</div>
        <div class="text-center position-absolute" style="width: 100%; transform: translateY(-50%); top: 40%">24</div>
        <div class="text-center position-absolute" style="width: 100%; transform: translateY(-50%); top: 50%">30</div>
        <div class="text-center position-absolute" style="width: 100%; transform: translateY(-50%); top: 70%">42</div>
        <div class="text-center position-absolute" style="width: 100%; transform: translateY(-80%); top: 100%">60</div>
      </div>
      <div class="d-flex flex-column position-relative" :style="{ top: '-' + props.width }">
        <div
          class="border position-relative"
          :class="clipping.right ? 'bg-red' : 'bg-surface'"
          style="box-sizing: content-box; top: -2px"
          :style="{ width: props.width, height: props.width }"
        ></div>
        <div class="border" :class="$style['meter-bar']" :style="{ width: props.width, height: props.height }">
          <div
            class="position-relative top-0 left-0 bg-surface"
            style="transition: height 40ms ease-out"
            :style="{
              width: props.width,
              height: (levels.right * 100) / -60 + '%',
            }"
          ></div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
  import { useTimeoutFn } from '@vueuse/core';
  import { onMounted, onUnmounted, ref } from 'vue';
  import { useApi } from '../../api';

  const api = useApi();

  const DECAY_STEP = 0.5;

  const props = withDefaults(
    defineProps<{
      kind?: 'master';
      width?: string;
      height?: string;
    }>(),
    {
      kind: 'master',
      width: '4px',
      height: '256px',
    },
  );

  const levels = ref({
    left: -60,
    right: -60,
  });

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

  const decayLoop = () => {
    if (levels.value.left > -60) {
      levels.value.left = Math.max(-60, levels.value.left - DECAY_STEP);
    }
    if (levels.value.right > -60) {
      levels.value.right = Math.max(-60, levels.value.right - DECAY_STEP);
    }

    if (levels.value.left > 0) {
      clipping.value.left = true;
      stopLeftClipReset();
      startLeftClipReset();
    }

    if (levels.value.right > 0) {
      clipping.value.right = true;
      stopRightClipReset();
      startRightClipReset();
    }

    animationFrameId = requestAnimationFrame(decayLoop);
  };

  onMounted(() => {
    api.listenLevelMeter((message) => {
      levels.value.left = Math.max(levels.value.left, Math.log10(message[0]) * 20);
      levels.value.right = Math.max(levels.value.right, Math.log10(message[1]) * 20);
    });
    decayLoop();
  });

  onUnmounted(() => {
    cancelAnimationFrame(animationFrameId);
  });
</script>

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
