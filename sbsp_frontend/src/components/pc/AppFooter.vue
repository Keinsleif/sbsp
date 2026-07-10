<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { mdiCog, mdiDockBottom, mdiDockRight, mdiServer, mdiSync } from '@mdi/js';
import { useUiState } from '../../stores/uiState.ts';
import { useShowModel } from '../../stores/showModel.ts';
import { useI18n } from 'vue-i18n';
import { message } from '@tauri-apps/plugin-dialog';
import { useApi } from '../../api';
import { useAssetResult } from '../../stores/assetResult';
import { computed, onMounted, ref } from 'vue';
import { check } from '@tauri-apps/plugin-updater';
import ButtonWrapper from '../wrapper/ButtonWrapper.vue';
import ProgressSpinnerWrapper from '../wrapper/ProgressSpinnerWrapper.vue';
import Select from 'primevue/select';

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
    if (val.value === 'view' && uiState.permission & 0b0001) {
      return true;
    }
    if (val.value === 'run' && uiState.permission & 0b0010) {
      return true;
    }
    if (val.value === 'edit' && uiState.permission & 0b0100) {
      return true;
    }
    return false;
  });
});

onMounted(() => {
  if (__IS_TAURI__ && Date.now() - uiState.lastUpdateCheckDate > 86400000) {
    check()
      .then((value) => {
        if (value != null) {
          isUpdateAvailable.value = true;
        }
        uiState.lastUpdateCheckDate = Date.now();
      })
      .catch((e) => {
        console.error(e);
      });
  }
});

const openSettings = async () => {
  uiState.isSettingsDialogOpen = true;
};

const openServerPanel = () => {
  api.host
    ?.getLicenseInfo()
    .then((info) => {
      if (info != null && info.edition === 'Pro') {
        uiState.isServerPanelOpen = true;
      } else {
        message(t('dialog.message.license.serverPanel'), {
          kind: 'info',
          title: t('dialog.message.license.proTitle'),
        });
      }
    })
    .catch((e) => console.error(e)); // Only fails in tauri internal. So no ui feedback needed.
};
</script>

<template>
  <div class="mr-0 ml-0 flex items-center py-1 border-t border-(--p-form-field-border-color)">
    <div class="flex items-center px-2">
      <Select
        v-model="uiState.mode"
        size="small"
        :options="modes"
        option-label="title"
        option-value="value"
        autocomplete="off"
      />
    </div>
    <button-wrapper
      class="ml-2"
      severity="secondary"
      size="small"
      :class="[isUpdateAvailable ? 'visible' : 'invisible']"
      :icon="mdiSync"
      :label="t('dialog.update.updatesAvailable')"
      @click="uiState.isUpdateDialogOpen = true"
    />
    <div class="mr-auto ml-auto whitespace-nowrap">
      {{ showModel.cueCount }} {{ t('main.footBar.cueCountSuffix') }}
    </div>
    <div class="flex items-center gap-2">
      <div
        :style="{ visibility: assetResult.processing.size > 0 ? 'visible' : 'hidden' }"
        class="flex items-center gap-2"
      >
        <progress-spinner-wrapper size="16px" />
        {{ t('main.footBar.processingIndicator', { size: assetResult.processing.size }) }}
      </div>
      <button-wrapper
        v-if="isHost"
        severity="secondary"
        :icon="mdiServer"
        @click="openServerPanel"
      />
      <button-wrapper
        :icon="mdiDockBottom"
        severity="secondary"
        rounded
        @click="uiState.toggleBottomTab"
      />
      <button-wrapper
        :icon="mdiDockRight"
        severity="secondary"
        rounded
        @click="uiState.toggleRightSidebar"
      />
      <button-wrapper
        :icon="mdiCog"
        severity="secondary"
        rounded
        @click="openSettings"
      />
    </div>
  </div>
</template>
