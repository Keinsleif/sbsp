<template>
  <div v-bind="props">
    <v-slider
      v-model="position"
      class="flex-grow-0"
      :readonly="activeTargetCue==null"
      :color="activeTargetCue?.status.startsWith('pre') ? 'warning' : 'primary'"
      min="0"
      :max="activeTargetCue?.duration || 0"
      @pointerdown="sliderChanging = true"
      @pointerup="onpointerup"
      hide-details
    />
    <div class="text-right">
      <span class="px-1">{{ secondsToFormat(activeTargetCue?.position || null) }}</span>
      /
      <span class="px-1">{{ activeTargetCue != null
        ? secondsToFormat(activeTargetCue.duration - activeTargetCue.position)
        : targetCue != null ? secondsToFormat(calculateDuration(targetCue.params, assetResult.getMetadata(targetCue.id)?.duration)) : '--:--.--' }}</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useAssetResult } from '../../stores/assetResult';
import { calculateDuration, secondsToFormat } from '../../utils';
import { useShowState } from '../../stores/showstate';
import { useShowModel } from '../../stores/showmodel';
import { storeToRefs } from 'pinia';
import { useApi } from '../../api';

const props = defineProps<{
  targetId: string | null;
}>();

const api = useApi();
const showModel = useShowModel();
const { getCueById } = storeToRefs(showModel);
const showState = useShowState();
const assetResult = useAssetResult();

const position = ref(0);
const sliderChanging = ref(false);

const targetCue = computed(() => {
  return props.targetId != null ? getCueById.value(props.targetId) : null;
});

const activeTargetCue = computed(() => {
  if (props.targetId == null) return null;
  const activeCue = showState.activeCues[props.targetId];
  if (activeCue == null) return null;
  return activeCue;
});

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
