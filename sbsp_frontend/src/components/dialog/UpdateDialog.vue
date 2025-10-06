<template>
  <v-dialog v-model="isUpdateDialogOpen" width="auto" @keydown.stop @afterEnter="checkUpdate">
    <v-sheet class="d-flex flex-column ga-4 pa-3" width="400px">
      <div class="d-flex flex-row align-center ga-4">
        <h2>Check for updates</h2>
        <v-progress-circular v-show="isCheckingUpdate" size="24" indeterminate></v-progress-circular>
      </div>
      <span :class="latestVersion == null ? 'text-red' : 'text-green'">
        {{ isCheckingUpdate ? '' : latestVersion == null ? 'No updates available.' : 'New updates available.' }}
      </span>
      <span>Current Version: {{ currentVersion != null ? currentVersion : '--' }}</span>
      <span>Latest Version: {{ latestVersion != null ? latestVersion : '--' }}</span>
      <v-progress-linear
        height="8"
        color="primary"
        :model-value="calculateProgress()"
        :indeterminate="total === 0n"
      ></v-progress-linear>
      <v-sheet class="mt-3 d-flex flex-row justify-end ga-2">
        <v-btn @click="isUpdateDialogOpen = false">Close</v-btn>
        <v-btn
          :disabled="latestVersion == null"
          :loading="progress != null && calculateProgress() == 100"
          color="primary"
          @click="installUpdate"
          >Install update</v-btn
        >
      </v-sheet>
    </v-sheet>
  </v-dialog>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { UpdateMetadata } from '../../types/UpdateMetadata';
import { Channel, invoke } from '@tauri-apps/api/core';
import { DownloadEvent } from '../../types/DownloadEvent';
import { getVersion } from '@tauri-apps/api/app';

const isCheckingUpdate = ref<boolean>(true);
const currentVersion = ref<string | null>(null);
const latestVersion = ref<string | null>(null);
const total = ref<bigint | null>(null);
const progress = ref<number | null>(null);

const isUpdateDialogOpen = defineModel<boolean>({ required: true });

const checkUpdate = () => {
  isCheckingUpdate.value = true;
  invoke<UpdateMetadata>('fetch_update')
    .then((value) => {
      isCheckingUpdate.value = false;
      if (value != null) {
        currentVersion.value = value.currentVersion;
        latestVersion.value = value.version;
      }
    })
    .catch((e) => {
      isCheckingUpdate.value = false;
      console.error(e);
    });
};

const installUpdate = () => {
  const downloadChannel = new Channel<DownloadEvent>();
  downloadChannel.onmessage = (message) => {
    switch (message.event) {
      case 'started':
        if (message.data.contentLength != null) {
          total.value = message.data.contentLength;
        } else {
          total.value = 0n;
        }
        break;
      case 'progress':
        progress.value = message.data.chunkLength;
        break;
      case 'finished':
        break;
    }
  };
  invoke('install_update', { onEvent: downloadChannel }).catch((e) => console.error(e));
};

const calculateProgress = (): number => {
  if (total.value === null || progress.value === null || total.value === 0n) {
    return 0;
  }
  return Number(BigInt(progress.value) / total.value) * 100;
};

onMounted(() => {
  getVersion()
    .then((value) => (currentVersion.value = value))
    .catch((e) => console.error(e));
});
</script>
