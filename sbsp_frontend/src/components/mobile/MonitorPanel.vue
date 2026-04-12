<template>
  <v-sheet class="h-100 pa-3">
    <div class="text-title-medium mb-2">
      {{ t('main.sideBar.activeCues') }}
    </div>
    <template
      v-for="(activeCue, cue_id) in showState.activeCues"
      :key="cue_id"
    >
      <v-sheet
        v-if="activeCue != null"
        class="d-flex flex-column border"
      >
        <v-sheet class="d-flex flex-row align-center justify-space-between pl-3 pr-3 pt-2">
          <span>{{ buildTitle(cue_id) }}</span>
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
              {{
                (['preWaiting', 'preWaitPaused'] as PlaybackStatus[]).includes(activeCue.status)
                  ? '-' + secondsToFormat(activeCue.duration - activeCue.position)
                  : secondsToFormat(activeCue.position)
              }}
            </v-list-item-subtitle>
          </v-list-item>
          <v-list-item density="compact">
            <v-list-item-subtitle>
              -{{
                (['preWaiting', 'preWaitPaused'] as PlaybackStatus[]).includes(activeCue.status)
                  ? '00:00.00' /* cue duration */
                  : secondsToFormat(activeCue.duration - activeCue.position)
              }}
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
  </v-sheet>
</template>

<script setup lang="ts">
import { storeToRefs } from 'pinia';
import { useShowModel } from '../../stores/showmodel';
import { buildCueName, secondsToFormat } from '../../utils';
import type { PlaybackStatus } from '../../types/PlaybackStatus';
import { mdiRepeat } from '@mdi/js';
import { useShowState } from '../../stores/showstate';
import { useI18n } from 'vue-i18n';

const showModel = useShowModel();
const { getCueById } = storeToRefs(showModel);
const showState = useShowState();
const { t } = useI18n();

const buildTitle = (cue_id: string) => {
  const activeCue = getCueById.value(cue_id);
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
};
</script>
