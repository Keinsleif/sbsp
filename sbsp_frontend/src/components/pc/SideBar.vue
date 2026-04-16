<template>
  <v-tabs
    v-model="uiState.sideBarTab"
    grow
    fixed-tabs
    density="compact"
  >
    <v-tab
      border
      density="compact"
      value="activeCues"
    >
      {{ t('main.sideBar.activeCues') }}
    </v-tab>
    <v-tab
      border
      density="compact"
      value="levels"
    >
      {{ t('main.sideBar.levels.title') }}
    </v-tab>
  </v-tabs>
  <v-tabs-window v-model="uiState.sideBarTab">
    <v-tabs-window-item
      value="activeCues"
      class="overflow-y-auto"
      style="font-size: 0.9em"
      transition="false"
      reverse-transition="false"
    >
      <template
        v-for="(activeCue, cue_id) in showState.activeCues"
        :key="cue_id"
      >
        <active-cue-item :active-cue="activeCue" />
      </template>
    </v-tabs-window-item>
    <v-tabs-window-item
      value="levels"
      transition="false"
      reverse-transition="false"
    >
      <level-meter
        v-if="uiState.sideBarTab == 'levels'"
        kind="master"
        height="400px"
      />
    </v-tabs-window-item>
  </v-tabs-window>
</template>

<script setup lang="ts">
import { useShowState } from '../../stores/showstate';
import { useUiState } from '../../stores/uistate';
import { useI18n } from 'vue-i18n';
import LevelMeter from '../input/LevelMeter.vue';
import ActiveCueItem from './ActiveCueItem.vue';

const { t } = useI18n();
const showState = useShowState();
const uiState = useUiState();
</script>
