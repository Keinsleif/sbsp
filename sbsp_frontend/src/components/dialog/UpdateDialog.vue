<template>
  <v-dialog
    v-model="isUpdateDialogOpen"
    width="auto"
    @keydown.stop
    @after-enter="checkUpdate"
    @contextmenu.prevent
  >
    <v-sheet
      class="d-flex flex-column ga-4 pa-3"
      width="400px"
    >
      <div class="d-flex flex-row align-center ga-4">
        <h2>{{ t('dialog.update.title') }}</h2>
        <v-progress-circular
          v-show="isCheckingUpdate"
          size="24"
          indeterminate="disable-shrink"
        />
      </div>
      <span :class="update == null ? 'text-red' : 'text-green'">
        {{
          isCheckingUpdate
            ? ''
            : update == null
              ? t('dialog.update.noUpdates')
              : t('dialog.update.updatesAvailable')
        }}
      </span>
      <span>{{ t('dialog.update.currentVersion') }}: {{ currentVersion != null ? currentVersion : '--' }}</span>
      <span>{{ t('dialog.update.latestVersion') }}: {{ update != null ? update.version : '--' }}</span>
      <v-progress-linear
        height="8"
        color="primary"
        :model-value="calculateProgress()"
        :indeterminate="total === 0"
      />
      <v-sheet class="mt-3 d-flex flex-row justify-end ga-2">
        <v-btn @click="isUpdateDialogOpen = false">
          {{ t('general.close') }}
        </v-btn>
        <v-btn
          :disabled="update == null"
          :loading="progress != null && calculateProgress() == 100"
          color="primary"
          @click="installUpdate"
        >
          {{ t('dialog.update.installUpdate') }}
        </v-btn>
      </v-sheet>
    </v-sheet>
  </v-dialog>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { check, Update } from '@tauri-apps/plugin-updater';
import { getVersion } from '@tauri-apps/api/app';

const { t } = useI18n();

const isCheckingUpdate = ref<boolean>(true);
const total = ref<number | null>(null);
const progress = ref<number | null>(null);

const isUpdateDialogOpen = defineModel<boolean>({ required: true });
const currentVersion = ref<string | null>(null);
const update = ref<Update | null>(null);

const checkUpdate = () => {
  isCheckingUpdate.value = true;
  check().then((value) => {
    isCheckingUpdate.value = false;
    update.value = value;
    if (value != null) {
      currentVersion.value = value.currentVersion;
    }
  }).catch((e) => {
    isCheckingUpdate.value = false;
    console.error(e);
  });
};

const installUpdate = () => {
  if (update.value != null) {
    update.value.downloadAndInstall((event) => {
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
    });
  }
};

const calculateProgress = (): number => {
  if (total.value === null || progress.value === null || total.value === 0) {
    return 0;
  }
  return progress.value / total.value * 100;
};

onMounted(() => {
  getVersion().then((version) => {
    currentVersion.value = version;
  }).catch(e => console.error(e));
});
</script>
