<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { computed, onMounted, ref } from 'vue';
import type { LicenseInformation } from '../../types/LicenseInformation';
import { mdiContentCopy, mdiReload } from '@mdi/js';
import { useI18n } from 'vue-i18n';
import { useApi } from '../../api';
import Dialog from 'primevue/dialog';
import ButtonWrapper from '../wrapper/ButtonWrapper.vue';
import FloatLabel from 'primevue/floatlabel';
import InputText from 'primevue/inputtext';

const { t, locale } = useI18n({ useScope: 'global' });
const api = useApi();
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
  api.host
    ?.getLicenseInfo()
    .then((value) => (licenseInfo.value = value))
    .catch((e) => console.error(e));
};

onMounted(() => {
  loadLicense();
});
</script>

<template>
  <Dialog
    v-model:visible="isLicenseDialogOpen"
    width="auto"
    :header="t('dialog.license.title')"
    @keydown.stop.esc="isLicenseDialogOpen = false"
    @contextmenu.prevent
  >
    <div class="flex flex-col p-4 gap-3 w-112.5">
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
      <div class="grow-0 flex flex-row">
        <FloatLabel
          variant="on"
          class="w-full"
        >
          <InputText
            id="on_label"
            class="w-full"
            readonly
            :model-value="licenseInfo != null ? licenseInfo.id : '-'"
            autocomplete="off"
          />
          <label for="on_label">Id</label>
        </FloatLabel>
        <button-wrapper
          :icon="mdiContentCopy"
          @click="onCopyId()"
          severity="secondary"
          rounded
        />
      </div>
    </div>
    <div class="grow-0 flex items-center ml-0 mr-0 w-full p-3 gap-3">
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
      width="600px"
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
