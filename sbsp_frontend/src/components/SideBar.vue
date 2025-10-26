<template>
  <v-tabs grow fixed-tabs v-model="uiState.sideBarTab" density="compact">
    <v-tab border density="compact" value="activeCues">Active Cues</v-tab>
    <v-tab border density="compact" value="levels">Levels</v-tab>
  </v-tabs>
  <v-tabs-window v-model="uiState.sideBarTab">
    <v-tabs-window-item value="activeCues" class="overflow-y-auto" transition="false" reverse-transition="false">
      <template v-for="(activeCue, cue_id) in showState.activeCues" :key="cue_id">
        <v-sheet v-if="activeCue != null" class="d-flex flex-column border">
          <v-sheet class="d-flex flex-row align-center justify-space-between pl-3 pr-3 pt-2">
            <span>{{ buildTitle(cue_id) }}</span>
            <v-icon
              v-if="activeCue.status != 'Stopping'"
              :icon="activeCue.params.type == 'audio' && activeCue.params.repeating === true ? mdiRepeat : undefined"
            ></v-icon>
            <v-progress-circular v-if="activeCue.status == 'Stopping'" indeterminate></v-progress-circular>
          </v-sheet>
          <v-sheet class="pa-0 d-flex flex-row justify-space-between">
            <v-list-item density="compact">
              <v-list-item-subtitle>
                {{
                  (['PreWaiting', 'PreWaitPaused'] as PlaybackStatus[]).includes(activeCue.status)
                    ? '-' + secondsToFormat(activeCue.duration - activeCue.position)
                    : secondsToFormat(activeCue.position)
                }}
              </v-list-item-subtitle>
            </v-list-item>
            <v-list-item density="compact">
              <v-list-item-subtitle>
                -{{
                  (['PreWaiting', 'PreWaitPaused'] as PlaybackStatus[]).includes(activeCue.status)
                    ? '00:00.00' /* cue duration */
                    : secondsToFormat(activeCue.duration - activeCue.position)
                }}
              </v-list-item-subtitle>
            </v-list-item>
          </v-sheet>
          <v-progress-linear
            :color="activeCue.status == 'Paused' || activeCue.status == 'Stopping' ? 'warning' : 'primary'"
            :model-value="activeCue != null ? (activeCue.position * 100) / activeCue.duration : 0"
            height="16"
          ></v-progress-linear>
        </v-sheet>
      </template>
    </v-tabs-window-item>
  </v-tabs-window>
</template>

<script setup lang="ts">
import { mdiRepeat } from '@mdi/js';
import { useShowModel } from '../stores/showmodel';
import { useShowState } from '../stores/showstate';
import { useUiState } from '../stores/uistate';
import type { PlaybackStatus } from '../types/PlaybackStatus';
import { buildCueName, secondsToFormat } from '../utils';

const showModel = useShowModel();
const showState = useShowState();
const uiState = useUiState();

const buildTitle = (cue_id: string) => {
  const activeCue = showModel.cues.find((cue) => cue.id == cue_id);
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
