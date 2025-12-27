<template>
  <v-dialog
    v-model="isLicenseDialogOpen"
    width="auto"
    @keydown.esc.stop="isLicenseDialogOpen = false"
    @keydown.stop
    @contextmenu.prevent
  >
    <v-sheet class="d-flex flex-column pa-4 ga-3" width="450px">
      <h2>{{ t('dialog.license.title') }}</h2>
      <div>
        {{ t('dialog.license.edition') }} :
        <span :class="edition == 'Free' ? '' : 'text-green'">{{ edition }}</span>
      </div>
      <div>
        {{ t('dialog.license.owner') }} :
        <span>{{ licenseInfo != null ? licenseInfo.owner : '-' }}</span>
      </div>
      <div>
        {{ t('dialog.license.issuedDate') }} :
        <span>{{ issuedDate }}</span>
      </div>
      <text-input
        :value="licenseInfo != null ? licenseInfo.id : '-'"
        label="Id"
        align-input="left"
        :append-inner-icon="mdiContentCopy"
        autocomplete="off"
        @click:appendInner="onCopyId()"
      ></text-input>
    </v-sheet>
    <v-footer class="flex-grow-0 d-flex align-center ml-0 mr-0 w-100 pa-3 ga-3">
      <v-btn :icon="mdiReload" density="comfortable" @click="loadLicense"></v-btn>
      <v-btn
        class="ml-auto"
        :text="t('dialog.license.activateLicense')"
        variant="outlined"
        @click="
          invoke<boolean>('activate_license', {})
            .then((isActivated) => {
              if (isActivated) {
                isLicenseActivateInfoDialogOpen = true;
              }
            })
            .catch((e) => console.error(e))
        "
      ></v-btn>
      <v-btn :text="t('general.close')" color="primary" @click="isLicenseDialogOpen = false"></v-btn>
    </v-footer>
    <v-dialog
      v-model="isLicenseActivateInfoDialogOpen"
      width="600px"
      @keydown.esc.stop="isLicenseActivateInfoDialogOpen = false"
      @keydown.stop
    >
      <v-card>
        <v-card-title class="d-flex flex-row text-success pa-4">
          <v-icon :icon="mdiCheckCircleOutline"></v-icon>
          <span class="font-weight-black">{{ t('dialog.license.success.title') }}</span>
        </v-card-title>
        <v-card-text class="bg-surface-light" style="white-space: pre-line">
          {{ t('dialog.license.success.message') }}
        </v-card-text>
        <v-card-actions>
          <v-btn :text="t('general.close')" color="primary" @click="isLicenseActivateInfoDialogOpen = false"></v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>
  </v-dialog>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { LicenseInformation } from '../../types/LicenseInformation';
import { invoke } from '@tauri-apps/api/core';
import TextInput from '../input/TextInput.vue';
import { mdiCheckCircleOutline, mdiContentCopy, mdiReload } from '@mdi/js';
import { useI18n } from 'vue-i18n';

const { t, locale } = useI18n({ useScope: 'global' });
const isLicenseDialogOpen = defineModel<boolean>();
const isLicenseActivateInfoDialogOpen = ref(false);
const licenseInfo = ref<LicenseInformation | null>(null);

const edition = computed(() => {
  return licenseInfo.value != null ? licenseInfo.value.edition : 'Free';
});

const issuedDate = computed(() => {
  if (licenseInfo.value != null) {
    const time = new Date(Number(licenseInfo.value.issue_time) * 1000);
    return time.toLocaleString(locale.value);
  } else {
    return '-';
  }
});

const onCopyId = () => {
  if (licenseInfo.value != null) {
    navigator.clipboard.writeText(licenseInfo.value.id);
  }
};

const loadLicense = () => {
  invoke<LicenseInformation | null>('get_license_info')
    .then((value) => (licenseInfo.value = value))
    .catch((e) => console.error(e));
};

onMounted(() => {
  loadLicense();
});
</script>
