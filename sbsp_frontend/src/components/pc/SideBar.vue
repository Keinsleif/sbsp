<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { useShowState } from '../../stores/showState';
import { useUiState } from '../../stores/uiState';
import { useI18n } from 'vue-i18n';
import LevelMeter from '../display/LevelMeter.vue';
import ActiveCueItem from '../ActiveCueItem.vue';
import TabList from 'primevue/tablist';
import Tabs from 'primevue/tabs';
import Tab from 'primevue/tab';
import TabPanels from 'primevue/tabpanels';
import TabPanel from 'primevue/tabpanel';

const { t } = useI18n();
const showState = useShowState();
const uiState = useUiState();
</script>

<template>
  <tabs
    v-model:value="uiState.sideBarTab"
    class="flex h-full flex-col border-l border-(--p-form-field-border-color)"
  >
    <tab-list>
      <tab value="activeCues">{{ t('main.sideBar.activeCues') }}</tab>
      <tab value="meter">{{ t('main.sideBar.meter.title') }}</tab>
    </tab-list>
    <tab-panels class="grow p-0 overflow-auto">
      <tab-panel value="activeCues">
        <template
          v-for="(activeCue, cue_id) in showState.activeCues"
          :key="cue_id"
        >
          <active-cue-item :active-cue="activeCue" :is-hidden="uiState.sideBarTab !== 'activeCues'" />
        </template>
      </tab-panel>
      <tab-panel value="meter" class="h-full">
        <level-meter
          v-if="uiState.sideBarTab == 'meter'"
          class="h-full"
          kind="master"
        />
      </tab-panel>
    </tab-panels>
  </tabs>
</template>
