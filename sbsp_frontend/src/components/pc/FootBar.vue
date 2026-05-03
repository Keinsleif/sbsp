<template>
  <v-sheet class="d-flex align-center ml-0 mr-0 w-100 position-relative">
    <v-sheet class="px-2 mr-auto d-flex align-center">
      <v-switch
        v-model="uiState.mode"
        hide-details
        :true-icon="mdiPencil"
        :false-icon="mdiEye"
        true-value="edit"
        false-value="run"
        density="compact"
      />
    </v-sheet>
    <v-btn
      v-show="isUpdateAvailable"
      class="position-absolute"
      style="left: 75px"
      variant="text"
      :prepend-icon="mdiSync"
      :text="t('dialog.update.updatesAvailable')"
      @click="uiState.isUpdateDialogOpen = true"
    />
    <v-sheet class="ml-auto mr-auto">
      {{ showModel.cueCount }} {{ t('main.footBar.cueCountSuffix') }}
    </v-sheet>
    <v-sheet class="mr-0 ml-auto d-flex align-center">
      <div v-show="assetResult.processing.size > 0" class="d-flex align-center ga-1 mr-4">
        <v-progress-circular
          indeterminate="disable-shrink"
          size="16"
        />
        {{ t('main.footBar.processingIndicator', { size: assetResult.processing.size }) }}
      </div>
      <v-btn
        v-if="side == 'host'"
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
import { mdiCog, mdiDockBottom, mdiDockRight, mdiEye, mdiPencil, mdiServer, mdiSync } from '@mdi/js';
import { useUiState } from '../../stores/uistate';
import { useShowModel } from '../../stores/showmodel';
import { useI18n } from 'vue-i18n';
import { message } from '@tauri-apps/plugin-dialog';
import { useApi, side, target } from '../../api';
import { useAssetResult } from '../../stores/assetResult';
import { onMounted, ref } from 'vue';
import { check } from '@tauri-apps/plugin-updater';

const { t } = useI18n();

const showModel = useShowModel();
const uiState = useUiState();
const api = useApi();
const assetResult = useAssetResult();

const isUpdateAvailable = ref(false);

onMounted(() => {
  if (target == 'tauri') {
    check().then((value) => {
      if (value != null) {
        isUpdateAvailable.value = true;
      }
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
      message(t('dialog.message.license.serverPanel'), { title: t('dialog.message.license.proTitle') });
    }
  });
};
</script>
