<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { useNow } from '@vueuse/core';
import { mdiFormatListBulleted, mdiMonitor, mdiRemote } from '@mdi/js';
import { ref, watch } from 'vue';
import ControlsPanel from './components/mobile/ControlsPanel.vue';
import MonitorPanel from './components/mobile/MonitorPanel.vue';
import CueList from './components/mobile/CueList.vue';
import { useI18n } from 'vue-i18n';
import { useUiState } from './stores/uiState';
import { PERMISSIONS } from './utils.ts';
import ButtonWrapper from './components/wrapper/ButtonWrapper.vue';
import ButtonGroup from 'primevue/buttongroup';

const uiState = useUiState();

const activeTab = ref('list');
const { t } = useI18n();
const time = useNow();

watch(() => uiState.permission, (newValue) => {
  if ((newValue & PERMISSIONS.CONTROL) === 0 && activeTab.value === 'controls') {
    activeTab.value = 'list';
  }
});
</script>

<template>
  <div class="flex flex-col h-full" @contextmenu.prevent>
    <div class="min-h-25">
      <div
        class="flex items-center border border-(--p-form-field-border-color) grow-0 h-full w-full"
      >
        <div
          class="flex items-end pl-3 pr-3 text-center justify-center grow"
          style="font-size: 4em; line-height: 1"
        >
          <span>{{ String(time.getHours()).padStart(2, '0') }}</span>:<span>{{ String(time.getMinutes()).padStart(2, '0') }}</span>.<span style="font-size: 32pt; line-height: 1">{{ String(time.getSeconds()).padStart(2, '0') }}</span>
        </div>
      </div>
    </div>
    <div class="grow shrink flex flex-col overflow-hidden">
      <div class="grow shrink flex flex-row overflow-hidden">
        <div v-show="activeTab === 'list'" class="flex h-full w-full overflow-hidden">
          <cue-list />
        </div>
        <div v-show="(uiState.permission & PERMISSIONS.CONTROL) != 0 && activeTab === 'controls'" class="h-full w-full">
          <controls-panel />
        </div>
        <div v-show="activeTab === 'monitor'" class="h-full w-full">
          <monitor-panel />
        </div>
      </div>
      <div class="grow-0 shrink-0">
        <button-group class="flex flex-row justify-center">
          <button-wrapper
            :icon="mdiFormatListBulleted"
            :severity="activeTab === 'list' ? 'primary' : 'secondary'"
            class="grow"
            :label="t('main.mobile.list')"
            @click="activeTab = 'list'"
          />
          <button-wrapper
            v-show="(uiState.permission & PERMISSIONS.CONTROL) != 0"
            :icon="mdiRemote"
            :severity="activeTab === 'controls' ? 'primary' : 'secondary'"
            class="grow"
            :label="t('main.mobile.controls')"
            @click="activeTab = 'controls'"
          />
          <button-wrapper
            :icon="mdiMonitor"
            :severity="activeTab === 'monitor' ? 'primary' : 'secondary'"
            class="grow"
            :label="t('main.mobile.active')"
            @click="activeTab = 'monitor'"
          />
        </button-group>
      </div>
    </div>
  </div>
</template>
