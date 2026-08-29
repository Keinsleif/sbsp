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
  if (supportedHW == null) return [];
  const id = deviceId.value || supportedHW.default;
  const device = supportedHW.devices[id];
  if (device == null) return [];

  const uniqueChannelCounts = new Set<number>();
  device.supportedConfigs.forEach((fc) => {
    uniqueChannelCounts.add(fc.channelCount);
  });
  return [
    { name: `${t('general.default')} (${device.defaultChannelCount})`, value: null },
    ...Array.from(uniqueChannelCounts)
      .sort((a, b) => a - b)
      .map((c) => ({ value: c, name: c.toString() })),
  ];
});
const channelCount = computed({
  get() {
    if (
      rawSettings.value.channelCount != null &&
      channelCounts.value.find((c) => c.value === rawSettings.value.channelCount) == null
    ) {
      return null;
    }
    return rawSettings.value.channelCount;
  },
  set(value) {
    rawSettings.value.channelCount = value;
  },
});

const sampleRates = computed(() => {
  const supportedHW = supportedHardware.value;
  if (supportedHW == null) return [];
  const id = deviceId.value || supportedHW.default;
  const device = supportedHW.devices[id];
  if (device == null) return [];

  const channels = channelCount.value || device.defaultChannelCount;

  const uniqueSampleRates = new Set<number>();
  for (const fc of device.supportedConfigs) {
    if (fc.channelCount === channels) {
      for (const srStr of Object.keys(fc.sampleRates)) {
        const sr = Number(srStr);
        if (!isNaN(sr)) {
          uniqueSampleRates.add(sr);
        }
      }
    }
  }
  return [
    { name: `${t('general.default')} (${device.defaultSampleRate / 1000} kHz)`, value: null },
    ...Array.from(uniqueSampleRates)
      .sort((a, b) => a - b)
      .map((sr) => ({ value: sr, name: (sr / 1000).toString() + ' kHz' })),
  ];
});
const sampleRate = computed({
  get() {
    if (
      rawSettings.value.sampleRate != null &&
      sampleRates.value.find((sr) => sr.value === rawSettings.value.sampleRate) == null
    ) {
      return null;
    }
    return rawSettings.value.sampleRate;
  },
  set(value) {
    rawSettings.value.sampleRate = value;
  },
});

const bufferSizes = computed(() => {
  const supportedHW = supportedHardware.value;
  if (supportedHW == null) return [];
  const id = deviceId.value || supportedHW.default;
  const device = supportedHW.devices[id];
  if (device == null) return [];

  const channels = channelCount.value || device.defaultChannelCount;
  const realSampleRate = sampleRate.value || device.defaultSampleRate;

  const uniqueBufferSizes = new Set<number>();
  for (const fc of device.supportedConfigs) {
    if (fc.channelCount === channels) {
      const supportedBufferSizes = fc.sampleRates[realSampleRate];
      if (supportedBufferSizes != null) {
        for (const bs of supportedBufferSizes) {
          uniqueBufferSizes.add(bs);
        }
      }
    }
  }
  return [
    { name: `${t('general.default')}`, value: null },
    ...Array.from(uniqueBufferSizes)
      .sort((a, b) => a - b)
      .map((bs) => ({ value: bs, name: bs.toString() + ' Frames' })),
  ];
});
const bufferSize = computed({
  get() {
    if (
      rawSettings.value.bufferSize != null &&
      bufferSizes.value.find((bs) => bs.value === rawSettings.value.bufferSize) == null
    ) {
      return null;
    }
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
