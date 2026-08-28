<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { mdiAlert } from '@mdi/js';
import { computed, onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { useApi } from '@/api';
import type { SupportedHardware } from '@/types/SupportedHardware';
import type { AudioHardwareSettings } from '@/types/AudioHardwareSettings';
import Message from 'primevue/message';
import PathIcon from '@/components/display/PathIcon.vue';
import SelectWrapper from '@/components/wrapper/SelectWrapper.vue';

const { t } = useI18n();
const api = useApi();
const rawSettings = defineModel<AudioHardwareSettings>({ required: true });
const supportedHardware = ref<SupportedHardware | null>(null);

onMounted(() => {
  if (api.host) {
    api.host.getHardware().then((value) => (supportedHardware.value = value));
  }
});

const devices = computed(() => {
  const supportedHW = supportedHardware.value;
  if (supportedHW != null) {
    const devices: { name: string; value: string | null }[] = [
      { name: t('general.default'), value: null },
    ];
    for (const dev in supportedHW.devices) {
      devices.push({ name: supportedHW.devices[dev]!.name, value: dev });
    }
    return devices;
  }
  return [];
});
const deviceId = computed({
  get() {
    if (
      rawSettings.value.deviceId != null &&
      devices.value.find((d) => d.value === rawSettings.value.deviceId) == null
    ) {
      return null;
    }
    return rawSettings.value.deviceId;
  },
  set(value) {
    rawSettings.value.deviceId = value;
  },
});

const channelCounts = computed(() => {
  const supportedHW = supportedHardware.value;
  if (supportedHW != null) {
    const id = deviceId.value || supportedHW.default;
    const device = supportedHW.devices[id];
    if (device != null) {
      const channels: { name: string; value: number | null }[] = [
        { name: `${t('general.default')} (${device.defaultChannelCount})`, value: null },
      ];
      device.supportedConfigs.forEach((fc) => {
        channels.push({ value: fc.channelCount, name: fc.channelCount.toString() });
      });
      return channels;
    }
  }
  return [];
});
const channelCount = computed({
  get() {
    return rawSettings.value.channelCount;
  },
  set(value) {
    rawSettings.value.channelCount = value;
  },
});

const sampleRates = computed(() => {
  const supportedHW = supportedHardware.value;
  if (supportedHW != null) {
    const id = deviceId.value || supportedHW.default;
    const device = supportedHW.devices[id];
    if (device != null) {
      const channels = channelCount.value || device.defaultChannelCount;
      let sampleRates: { name: string; value: number | null }[] = [
        { name: `${t('general.default')} (${device.defaultSampleRate / 1000} kHz)`, value: null },
      ];
      for (const fc of device.supportedConfigs) {
        if (fc.channelCount === channels) {
          sampleRates = sampleRates.concat(
            fc.sampleRates.map((sr) => ({ value: sr, name: (sr / 1000).toString() + ' kHz' })),
          );
        }
      }
      return sampleRates;
    }
  }
  return [];
});
const sampleRate = computed({
  get() {
    return rawSettings.value.sampleRate;
  },
  set(value) {
    rawSettings.value.sampleRate = value;
  },
});

const bufferSizes = computed(() => {
  const supportedHW = supportedHardware.value;
  if (supportedHW != null) {
    const id = deviceId.value || supportedHW.default;
    const device = supportedHW.devices[id];
    if (device != null) {
      const channels = channelCount.value || device.defaultChannelCount;
      let bufferSizes: { name: string; value: number | null }[] = [
        { name: `${t('general.default')}`, value: null },
      ];
      for (const fc of device.supportedConfigs) {
        if (fc.channelCount === channels) {
          bufferSizes = bufferSizes.concat(
            fc.bufferSizes.map((bs) => ({ value: bs, name: bs.toString() + ' Frames' })),
          );
        }
      }
      return bufferSizes;
    }
  }
  return [];
});
const bufferSize = computed({
  get() {
    return rawSettings.value.bufferSize;
  },
  set(value) {
    rawSettings.value.bufferSize = value;
  },
});

const fallbackInfo = computed(() => {
  const info = {
    device: false,
    config: false,
  };
  const settings = rawSettings.value;

  if (
    devices.value.length !== 0 &&
    settings.deviceId != null &&
    devices.value.find((d) => d.value === settings.deviceId) == null
  ) {
    info.device = true;
  }
  if (
    channelCounts.value.length !== 0 &&
    settings.channelCount != null &&
    channelCounts.value.find((c) => c.value === settings.channelCount) == null
  ) {
    info.config = true;
  }
  if (
    sampleRates.value.length !== 0 &&
    settings.sampleRate != null &&
    sampleRates.value.find((s) => s.value === settings.sampleRate) == null
  ) {
    info.config = true;
  }
  if (
    bufferSizes.value.length !== 0 &&
    settings.bufferSize != null &&
    bufferSizes.value.find((b) => b.value === settings.bufferSize) == null
  ) {
    info.config = true;
  }

  return info;
});
</script>

<template>
  <div>
    <Message
      class="shrink-0"
      severity="error"
    >
      <template #icon="innerProps">
        <path-icon
          :class="innerProps.class"
          :icon="mdiAlert"
        />
      </template>
      {{ t('dialog.settings.global.audioHardware.warning') }}
    </Message>
  </div>
  <h2>
    {{ t('dialog.settings.global.audioHardware.device') }}
    <path-icon
      v-show="fallbackInfo.device"
      class="text-orange-500"
      :icon="mdiAlert"
      :title="t('dialog.settings.global.audioHardware.deviceIdFallbackTitle')"
    />
  </h2>
  <select-wrapper
    v-model="deviceId"
    :label="t('dialog.settings.global.audioHardware.device')"
    :items="devices"
  />
  <h2>
    {{ t('dialog.settings.global.audioHardware.deviceConfig') }}
    <path-icon
      v-show="fallbackInfo.config"
      class="text-orange-500"
      :icon="mdiAlert"
      :title="t('dialog.settings.global.audioHardware.configFallbackTitle')"
    />
  </h2>
  <select-wrapper
    v-model="channelCount"
    :label="t('dialog.settings.global.audioHardware.channelCount')"
    :items="channelCounts"
  />
  <select-wrapper
    v-model="sampleRate"
    :label="t('dialog.settings.global.audioHardware.sampleRate')"
    :items="sampleRates"
  />
  <select-wrapper
    v-model="bufferSize"
    :label="t('dialog.settings.global.audioHardware.bufferSize')"
    :items="bufferSizes"
  />
</template>
