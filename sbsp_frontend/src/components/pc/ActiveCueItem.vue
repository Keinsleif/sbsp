<template>
  <v-sheet
    class="d-flex flex-column border"
  >
    <v-sheet class="d-flex flex-row align-center justify-space-between pl-3 pr-3 pt-2">
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
    </v-sheet>
    <v-sheet class="pa-0 d-flex flex-row justify-space-between">
      <v-list-item density="compact">
        <v-list-item-subtitle>
          {{ elapsed }}
        </v-list-item-subtitle>
      </v-list-item>
      <v-list-item density="compact">
        <v-list-item-subtitle>
          -{{ remain }}
        </v-list-item-subtitle>
      </v-list-item>
    </v-sheet>
    <v-progress-linear
      :color="activeCue.status == 'paused' || activeCue.status == 'stopping' ? 'warning' : 'primary'"
      style="transition: none;"
      :model-value="activeCue != null ? (activeCue.position * 100) / activeCue.duration : 0"
      height="16"
    />
  </v-sheet>
</template>

<script setup lang="ts">
import { storeToRefs } from 'pinia';
import { useShowModel } from '../../stores/showmodel';
import type { ActiveCue } from '../../types/ActiveCue';
import { computed } from 'vue';
import { buildCueName, secondsToFormat } from '../../utils';
import type { PlaybackStatus } from '../../types/PlaybackStatus';
import { mdiRepeat } from '@mdi/js';

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

const elapsed = computed(() => {
  return (['preWaiting', 'preWaitPaused'] as PlaybackStatus[]).includes(props.activeCue.status)
    ? '-' + secondsToFormat(props.activeCue.duration - props.activeCue.position)
    : secondsToFormat(props.activeCue.position);
});
const remain = computed(() => {
  return (['preWaiting', 'preWaitPaused'] as PlaybackStatus[]).includes(props.activeCue.status)
    ? '00:00.00' /* cue duration */
    : secondsToFormat(props.activeCue.duration - props.activeCue.position);
});
</script>
