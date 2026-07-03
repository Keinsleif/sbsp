<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { computed, onMounted, ref } from 'vue';
import type { LicenseInformation } from '../../types/LicenseInformation';
import { mdiReload } from '@mdi/js';
import { useI18n } from 'vue-i18n';
import { useApi } from '../../api';
import Dialog from 'primevue/dialog';
import ButtonWrapper from '../wrapper/ButtonWrapper.vue';
import CopyTextInput from '../input/CopyTextInput.vue';
import { $dt } from '@primeuix/themes';
import { useToast } from 'primevue/usetoast';

const { t, locale } = useI18n({ useScope: 'global' });
const api = useApi();
const toast = useToast();
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

const loadLicense = () => {
  api.host
    ?.getLicenseInfo()
    .then((value) => (licenseInfo.value = value))
    .catch((e) => {
      console.error(e);
      toast.add({ severity: 'error', summary: 'Failed to load License', detail: e.toString(), life: 3000 });
    });
};

onMounted(() => {
  loadLicense();
});
</script>

<template>
  <Dialog
    v-model:visible="isLicenseDialogOpen"
    class="w-auto"
    :header="t('dialog.license.title')"
    @keydown.stop.esc="isLicenseDialogOpen = false"
    @contextmenu.prevent
  >
    <div class="flex w-md flex-col gap-3 p-4">
      <div>
        {{ t('dialog.license.edition') }} :
        <span
          :style="{
            color: edition == 'Free' ? '' : $dt('green.500').variable,
          }"
          >{{ edition }}</span
        >
      </div>
      <div>
        {{ t('dialog.license.owner') }} :
        <span>{{ licenseInfo != null ? licenseInfo.owner : '-' }}</span>
      </div>
      <div>
        {{ t('dialog.license.issuedDate') }} :
        <span>{{ issuedDate }}</span>
      </div>
      <copy-text-input
        class="w-full"
        readonly
        label="Id"
        :model-value="licenseInfo != null ? licenseInfo.id : '-'"
        :disabled="licenseInfo == null"
      />
    </div>
    <div class="mr-0 ml-0 flex w-full grow-0 items-center gap-3 p-3">
      <button-wrapper
        :icon="mdiReload"
        severity="secondary"
        rounded
        @click="loadLicense"
      />
      <button-wrapper
        class="ml-auto"
        :label="t('dialog.license.activateLicense')"
        variant="outlined"
        @click="
          api.host
            ?.activateLicense()
            .then((isActivated) => {
              if (isActivated) {
                isLicenseActivateInfoDialogOpen = true;
              }
            })
            .catch((e) => console.error(e))
        "
      />
    </div>
    <Dialog
      v-model:visible="isLicenseActivateInfoDialogOpen"
      class="w-150"
      :header="t('dialog.license.success.title')"
      @keydown.stop.esc="isLicenseActivateInfoDialogOpen = false"
    >
      <div
        class="bg-surface-light"
        style="white-space: pre-line"
      >
        {{ t('dialog.license.success.message') }}
      </div>
    </Dialog>
  </Dialog>
</template>
