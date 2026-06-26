<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import Dialog from 'primevue/dialog';
import { computed, onMounted, ref, toRaw, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { check, Update } from '@tauri-apps/plugin-updater';
import { getVersion } from '@tauri-apps/api/app';
import ProgressSpinnerWrapper from '../wrapper/ProgressSpinnerWrapper.vue';
import ProgressBar from 'primevue/progressbar';
import ButtonWrapper from '../wrapper/ButtonWrapper.vue';
import { useToast } from 'primevue/usetoast';

const { t } = useI18n();
const toast = useToast();

const isCheckingUpdate = ref<boolean>(true);
const total = ref<number | null>(null);
const progress = ref<number | null>(null);

const isUpdateDialogOpen = defineModel<boolean>({ required: true });
const currentVersion = ref<string | null>(null);
const update = ref<Update | null>(null);

const checkUpdate = () => {
  isCheckingUpdate.value = true;
  check()
    .then((value) => {
      isCheckingUpdate.value = false;
      update.value = value;
      if (value != null) {
        currentVersion.value = value.currentVersion;
      }
    })
    .catch((e) => {
      isCheckingUpdate.value = false;
      console.error(e);
    });
};

const installUpdate = () => {
  if (update.value != null) {
    toRaw(update.value).downloadAndInstall((event) => {
      switch (event.event) {
        case 'Started':
          if (event.data.contentLength != null) {
            total.value = event.data.contentLength;
          } else {
            total.value = 0;
          }
          break;
        case 'Progress':
          if (progress.value == null) {
            progress.value = event.data.chunkLength;
          } else {
            progress.value += event.data.chunkLength;
          }
          break;
        case 'Finished':
          console.log('download finished');
          break;
      }
    }).catch((e) => {
      console.error(e);
      toast.add({ severity: 'error', summary: t('notification.updateFailed'), detail: e, life: 3000 });
      total.value = null;
      progress.value = null;
    }); // If success install, app will be restarted by tauri.
  }
};

const progressBarValue = computed(() => {
  if (total.value === null || progress.value === null || total.value === 0) {
    return 0;
  }
  return (progress.value / total.value) * 100;
});

watch(isUpdateDialogOpen, (value) => {
  if (value) {
    checkUpdate();
  }
});

onMounted(() => {
  getVersion()
    .then((version) => {
      currentVersion.value = version;
    })
    .catch((e) => console.error(e));
});
</script>

<template>
  <Dialog
    v-model:visible="isUpdateDialogOpen"
    class="w-auto"
    :header="t('dialog.update.title')"
    @keydown.stop
    @contextmenu.prevent
  >
    <div class="flex flex-col gap-4 p-3 w-100">
      <div class="flex flex-row items-center justify-start gap-4">
        <span :class="update == null ? 'text-red-500' : 'text-green-600'">
          {{
            isCheckingUpdate
              ? t('dialog.update.checking')
              : update == null
                ? t('dialog.update.noUpdates')
                : t('dialog.update.updatesAvailable')
          }}
        </span>
        <progress-spinner-wrapper
          v-show="isCheckingUpdate"
          class="grow-0 m-0"
          size="16px"
        />
      </div>
      <span
        >{{ t('dialog.update.currentVersion') }}:
        {{ currentVersion != null ? currentVersion : '--' }}</span
      >
      <span
        >{{ t('dialog.update.latestVersion') }}: {{ update != null ? update.version : '--' }}</span
      >
      <ProgressBar
        :show-value="false"
        :value="progressBarValue"
        :mode="total === 0 ? 'indeterminate' : 'determinate'"
      />
      <div class="mt-3 flex flex-row justify-end gap-2">
        <ButtonWrapper
          :label="t('dialog.update.installUpdate')"
          :disabled="update == null"
          :loading="progress != null"
          severity="primary"
          @click="installUpdate"
        />
      </div>
    </div>
  </Dialog>
</template>
