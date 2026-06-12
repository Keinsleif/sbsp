<template>
  <v-sheet class="d-flex align-center ml-0 mr-0 w-100">
    <v-sheet class="px-2 d-flex align-center">
      <v-select
        v-model="uiState.mode"
        hide-details
        :items="modes"
        variant="outlined"
        density="compact"
        autocomplete="off"
      />
    </v-sheet>
    <v-btn
      class="ml-2"
      :style="{ 'visibility': isUpdateAvailable ? 'visible' : 'hidden' }"
      variant="text"
      :prepend-icon="mdiSync"
      :text="t('dialog.update.updatesAvailable')"
      @click="uiState.isUpdateDialogOpen = true"
    />
    <v-sheet class="ml-auto mr-auto text-no-wrap">
      {{ showModel.cueCount }} {{ t('main.footBar.cueCountSuffix') }}
    </v-sheet>
    <v-sheet class="d-flex align-center">
      <div
        :style="{ 'visibility': assetResult.processing.size > 0 ? 'visible' : 'hidden' }"
        class="d-flex align-center ga-1 mr-2"
      >
        <v-progress-circular
          indeterminate="disable-shrink"
          size="16"
        />
        {{ t('main.footBar.processingIndicator', { size: assetResult.processing.size }) }}
      </div>
      <v-btn
        v-if="isHost"
        :icon="mdiServer"
        size="small"
        variant="text"
        @click="openServerPanel"
      />
      <v-btn
        :icon="mdiDockBottom"
        size="small"
        variant="text"
        @click="uiState.toggleBottomTab"
      />
      <v-btn
        :icon="mdiDockRight"
        size="small"
        variant="text"
        @click="uiState.toggleRightSidebar"
      />
      <v-btn
        :icon="mdiCog"
        size="small"
        variant="text"
        @click="openSettings"
      />
    </v-sheet>
  </v-sheet>
</template>

<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { mdiCog, mdiDockBottom, mdiDockRight, mdiServer, mdiSync } from '@mdi/js';
import { useUiState } from '../../stores/uistate';
import { useShowModel } from '../../stores/showmodel';
import { useI18n } from 'vue-i18n';
import { message } from '@tauri-apps/plugin-dialog';
import { useApi } from '../../api';
import { useAssetResult } from '../../stores/assetResult';
import { computed, onMounted, ref } from 'vue';
import { check } from '@tauri-apps/plugin-updater';

const isHost = __IS_HOST__;

const { t } = useI18n();

const showModel = useShowModel();
const uiState = useUiState();
const api = useApi();
const assetResult = useAssetResult();

const isUpdateAvailable = ref(false);

const ALL_MODES = computed(() => [
  { title: t('main.footBar.modes.view'), value: 'view' },
  { title: t('main.footBar.modes.run'), value: 'run' },
  { title: t('main.footBar.modes.edit'), value: 'edit' },
]);

const modes = computed(() => {
  return ALL_MODES.value.filter((val) => {
    if (uiState.permission == null) return false;
    if (val.value == 'view' && uiState.permission & 0b0001) {
      return true;
    }
    if (val.value == 'run' && uiState.permission & 0b0010) {
      return true;
    }
    if (val.value == 'edit' && uiState.permission & 0b0100) {
      return true;
    }
    return false;
  });
});

onMounted(() => {
  if (__IS_TAURI__ && Date.now() - uiState.lastUpdateCheckDate > 86400000) {
    check().then((value) => {
      if (value != null) {
        isUpdateAvailable.value = true;
      }
      uiState.lastUpdateCheckDate = Date.now();
    }).catch((e) => {
      console.error(e);
    });
  }
});

const openSettings = async () => {
  uiState.isSettingsDialogOpen = true;
};

const openServerPanel = () => {
  api.host?.getLicenseInfo().then((info) => {
    if (info != null && info.edition == 'Pro') {
      uiState.isServerPanelOpen = true;
    } else {
      message(t('dialog.message.license.serverPanel'), { kind: 'info', title: t('dialog.message.license.proTitle') });
    }
  });
};
</script>
