<template>
  <v-tabs grow fixed-tabs v-model="uiState.sideBarTab" density="compact">
    <v-tab border density="compact" value="activeCues">Active Cues</v-tab>
    <v-tab border density="compact" value="levels">Levels</v-tab>
  </v-tabs>
  <v-tabs-window v-model="uiState.sideBarTab">
    <v-tabs-window-item
      value="activeCues"
      class="overflow-y-auto"
      transition="false"
      reverse-transition="false"
    >
      <v-card v-for="(activeCue, cue_id) in showState.activeCues" :key="cue_id" class="border">
        <v-card-title class="text-subtitle-1 pb-0">
          {{ showModel.cues.find((cue) => cue.id == cue_id)?.number+ "・" + showModel.cues.find((cue) => cue.id == cue_id)?.name }}
        </v-card-title>
        <v-card-subtitle class="pa-0 d-flex justify-space-between">
          <v-list-item density="compact">
            <v-list-item-subtitle>
              {{ activeCue != null ? activeCue.status in ["PreWaiting", "PreWaitPaused"] ? "-"+secondsToFormat(activeCue.duration - activeCue.position) : secondsToFormat(activeCue.position) : "" }}
            </v-list-item-subtitle>
          </v-list-item>
          <v-list-item density="compact">
            <v-list-item-subtitle>
              -{{ activeCue != null ? activeCue.status in ["PreWaiting", "PreWaitPaused"] ? "00:00.00" /* cue duration */ : secondsToFormat(activeCue.duration - activeCue.position) : "" }}
            </v-list-item-subtitle>
          </v-list-item>
        </v-card-subtitle>
        <v-progress-linear
          :color="activeCue?.status == 'Paused' ? 'warning' : 'primary'"
          :model-value="activeCue != null ? activeCue?.position * 100 / activeCue?.duration : 0"
          height="8"
        ></v-progress-linear>
      </v-card>
    </v-tabs-window-item>
  </v-tabs-window>
</template>

<script setup lang="ts">
import { useShowModel } from '../stores/showmodel';
import { useShowState } from '../stores/showstate';
import { useUiState } from '../stores/uistate';
import { secondsToFormat } from '../utils';

const showModel = useShowModel();
const showState = useShowState();
const uiState = useUiState();
</script>
