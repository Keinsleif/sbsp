<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { computed, onMounted, ref, toRaw, watch } from 'vue';
import { useShowModel } from '../../stores/showModel';
import type { ShowSettings } from '../../types/ShowSettings';
import HotkeyInput from '../input/HotkeyInput.vue';
import TextInput from '../input/TextInput.vue';
import { useUiSettings } from '../../stores/uiSettings';
import type { GlobalHostSettings } from '../../types/GlobalHostSettings';
import type { GlobalRemoteSettings } from '../../types/GlobalRemoteSettings';
import { useI18n } from 'vue-i18n';
import { useApi } from '../../api';
import TemplateSettings from './settings/TemplateSettings.vue';
import type { SupportedHardware } from '../../types/SupportedHardware';
import { useShowState } from '../../stores/showState';
import { message } from '@tauri-apps/plugin-dialog';
import type { AudioHardwareSettings } from '../../types/AudioHardwareSettings';
import Drawer from 'primevue/drawer';
import ButtonWrapper from '../wrapper/ButtonWrapper.vue';
import NumberInput from '../input/NumberInput.vue';
import CheckboxWrapper from '../wrapper/CheckboxWrapper.vue';
import PathIcon from '../display/PathIcon.vue';
import { mdiAlert } from '@mdi/js';
import SelectWrapper from '../wrapper/SelectWrapper.vue';
import Divider from 'primevue/divider';
import Message from 'primevue/message';

const { t } = useI18n();
const api = useApi();
const showModel = useShowModel();
const uiSettings = useUiSettings();
const showState = useShowState();

const isSettingsDialogOpen = defineModel<boolean>({ required: true });

const tab = ref('showGeneral');
const showModelName = ref<string>(showModel.name);
const editingSettings = ref<{
  show: ShowSettings;
  global: GlobalHostSettings | GlobalRemoteSettings;
}>({ show: structuredClone(toRaw(showModel.settings)), global: uiSettings.clone() });
const supportedHardware = ref<SupportedHardware | null>(null);

const tabItems = [
  { type: 'tab', value: 'preset', label: t('dialog.settings.tab.preset') },
  { type: 'group', value: 'showModel', label: t('dialog.settings.tab.category.inThisShowModel') },
  { type: 'tab', value: 'showGeneral', label: t('dialog.settings.tab.general') },
  { type: 'tab', value: 'audioLogic', label: t('dialog.settings.tab.audioLogic') },
  { type: 'tab', value: 'remote', label: t('dialog.settings.tab.remote') },
  { type: 'group', value: 'global',label: t('dialog.settings.tab.category.global') },
  { type: 'tab', value: 'globalGeneral', label: t('dialog.settings.tab.general') },
  { type: 'tab', value: 'appearance', label: t('dialog.settings.tab.appearance') },
  ...(__IS_HOST__ ? [{ type: 'tab', value: 'audioHardware', label: t('dialog.settings.tab.audioHardware') }] : []),
  { type: 'tab', value: 'hotkey', label: t('dialog.settings.tab.hotkey') },
  { type: 'tab', value: 'template', label: t('dialog.settings.tab.template') },
  { type: 'tab', value: 'nameFormat', label: t('dialog.settings.tab.nameFormat') },
];

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
const channelCounts = computed(() => {
  const supportedHW = supportedHardware.value;
  if ('audio' in editingSettings.value.global && supportedHW != null) {
    const id = editingSettings.value.global.audio.deviceId || supportedHW.default;
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
const sampleRates = computed(() => {
  const supportedHW = supportedHardware.value;
  if ('audio' in editingSettings.value.global && supportedHW != null) {
    const id = editingSettings.value.global.audio.deviceId || supportedHW.default;
    const device = supportedHW.devices[id];
    if (device != null) {
      const channels =
        editingSettings.value.global.audio.channelCount || device.defaultChannelCount;
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
const bufferSizes = computed(() => {
  const supportedHW = supportedHardware.value;
  if ('audio' in editingSettings.value.global && supportedHW != null) {
    const id = editingSettings.value.global.audio.deviceId || supportedHW.default;
    const device = supportedHW.devices[id];
    if (device != null) {
      const channels =
        editingSettings.value.global.audio.channelCount || device.defaultChannelCount;
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

watch(
  () => showModel.settings,
  (newSettings) => {
    editingSettings.value.show = structuredClone(toRaw(newSettings));
  },
);

watch(
  () => showModel.name,
  (newName) => {
    showModelName.value = newName;
  },
);

watch(
  () => uiSettings.settings,
  () => {
    editingSettings.value.global = uiSettings.clone();
  },
);

watch(isSettingsDialogOpen, (newState) => {
  if (newState) {
    editingSettings.value = {
      show: structuredClone(toRaw(showModel.settings)),
      global: uiSettings.clone(),
    };
  }
});

onMounted(() => {
  if (api.host) {
    api.host.getHardware().then((value) => (supportedHardware.value = value));
  }
});

const saveSettings = async (): Promise<boolean> => {
  if (
    __IS_HOST__ &&
    'audio' in uiSettings.settings &&
    'audio' in editingSettings.value.global &&
    !isEqualAudioHardware(editingSettings.value.global.audio, uiSettings.settings.audio)
  ) {
    const activeIds = Object.keys(showState.activeCues);
    let hasActiveAudioCue = false;
    for (const id of activeIds) {
      if (showModel.getCueById(id)?.params.type === 'audio') {
        hasActiveAudioCue = true;
        break;
      }
    }
    if (hasActiveAudioCue) {
      const result = await message(t('dialog.settings.global.audioHardware.saveWarning'), {
        buttons: 'OkCancel',
        kind: 'warning',
        title: t('general.warning'),
      });
      if (result === 'Cancel') {
        return false;
      }
    }
  }
  api.updateShowSettings(editingSettings.value.show);
  api.updateModelName(showModelName.value);
  uiSettings.update(editingSettings.value.global);
  return true;
};

const isEqualAudioHardware = (a: AudioHardwareSettings, b: AudioHardwareSettings): boolean => {
  return (
    a.deviceId === b.deviceId &&
    a.channelCount === b.channelCount &&
    a.sampleRate === b.sampleRate &&
    a.bufferSize === b.bufferSize
  );
};

const recallMusicBeePreset = () => {
  editingSettings.value.global.hotkey.playback = {
    go: 'Enter',
    load: 'L',
    pauseAndResume: 'Space',
    pauseAll: '[',
    resumeAll: ']',
    stop: 'Backspace',
    stopAll: 'Escape',
    seekForward: null,
    seekBackward: null,
  };
  editingSettings.value.global.hotkey.audioAction = {
    toggleRepeat: 'R',
  };
  editingSettings.value.global.general.advanceCursorWhenGo = false;
};

const recallQLabPreset = () => {
  editingSettings.value.global.hotkey.playback = {
    go: 'Space',
    load: 'L',
    pauseAndResume: 'P',
    pauseAll: '[',
    resumeAll: ']',
    stop: 'S',
    stopAll: 'Escape',
    seekForward: null,
    seekBackward: null,
  };
  editingSettings.value.global.hotkey.audioAction = {
    toggleRepeat: 'R',
  };
  editingSettings.value.global.general.advanceCursorWhenGo = true;
};
</script>

<template>
  <Drawer
    v-model:visible="isSettingsDialogOpen"
    position="full"
    @keydown.stop.esc="isSettingsDialogOpen = false"
    @contextmenu.prevent
  >
    <div class="flex h-full w-full flex-col">
      <div class="flex w-full grow flex-row">
        <div
          class="flex min-w-50 grow-0 flex-col border border-(--p-form-field-border-color) p-2"
          :class="$style['tablist']"
        >
          <template v-for="tabItem in tabItems" :key="tabItem.value">
            <button
              v-if="tabItem.type === 'tab'"
              class="text-left px-2 py-1 transition-all duration-200"
              :class="tab === tabItem.value ? $style['selected-category'] : ''"
              @click="tab = tabItem.value"
            >
              {{ tabItem.label }}
            </button>
            <div
              v-else
              class="border-y text-gray-500 dark:text-gray-400 my-2 py-1 px-3"
            >
              {{ tabItem.label  }}
            </div>
          </template>
        </div>
        <div class="h-full grow">
          <div
            v-show="tab === 'preset'"
            class="p-4"
          >
            <h2 class="mb-4">
              {{ t('dialog.settings.preset.hotkeyCursor.title') }}
            </h2>
            <div class="flex flex-row items-center gap-3">
              <button-wrapper
                class="w-30"
                severity="primary"
                label="MusicBee"
                @click="recallMusicBeePreset"
              />
              <span>{{ t('dialog.settings.preset.hotkeyCursor.musicBee.description') }}</span>
            </div>
            <divider class="mt-4 mb-4" />
            <div class="flex flex-row items-center gap-3">
              <button-wrapper
                class="w-30"
                severity="primary"
                label="QLab"
                @click="recallQLabPreset"
              />
              <span>{{ t('dialog.settings.preset.hotkeyCursor.qLab.description') }}</span>
            </div>
          </div>
          <div
            v-show="tab === 'showGeneral'"
            class="flex flex-col gap-4 p-4"
          >
            <text-input
              v-model="showModelName"
              class="w-125"
              :label="t('dialog.settings.show.general.showModelName')"
            />
            <text-input
              v-model="editingSettings.show.general.copyAssetsDestination"
              class="w-125"
              :label="t('dialog.settings.show.general.assetsDirectory.title')"
              :help="t('dialog.settings.show.general.assetsDirectory.description')"
            />
          </div>
          <div
            v-show="tab === 'audioLogic'"
            class="flex flex-col gap-4 p-4"
          >
            <checkbox-wrapper
              v-model="editingSettings.show.audio.monoOutput"
              :label="t('dialog.settings.show.audioLogic.monoOutput')"
            />
            <number-input
              v-model="editingSettings.show.audio.lufsTarget"
              class="w-80"
              :label="t('dialog.settings.show.audioLogic.targetLufs')"
              suffix="LUFS"
              :precision="2"
            />
          </div>
          <div
            v-show="tab === 'remote'"
            class="flex flex-col gap-4 p-4"
          >
            <checkbox-wrapper
              v-model="editingSettings.show.remote.lockCursorToSelection"
              :label="t('dialog.settings.show.remote.lockCursorToSelection')"
            />
          </div>
          <div
            v-show="tab === 'globalGeneral'"
            class="flex flex-col gap-4 p-4"
          >
            <checkbox-wrapper
              v-model="editingSettings.global.general.advanceCursorWhenGo"
              :label="t('dialog.settings.global.general.advanceCursorWhenGo')"
              hide-details
            />
            <checkbox-wrapper
              v-model="editingSettings.global.general.lockCursorToSelection"
              :label="t('dialog.settings.global.general.lockCursorToSelection')"
              hide-details
            />
            <checkbox-wrapper
              v-model="editingSettings.global.general.copyAssetsWhenAdd"
              :label="t('dialog.settings.global.general.copyAssetsWhenAdd')"
              hide-details
            />
            <number-input
              v-model="editingSettings.global.general.seekAmount"
              class="w-50"
              :min="0"
              :label="t('dialog.settings.global.general.seekAmount')"
              :precision="2"
              show-buttons
            />
          </div>
          <div
            v-show="tab === 'appearance'"
            class="flex flex-col gap-4 p-4"
          >
            <select-wrapper
              v-model="editingSettings.global.appearance.language"
              class="w-80"
              :label="t('dialog.settings.global.appearance.language')"
              :items="[
                { value: null, name: t('dialog.settings.global.appearance.systemLanguage') },
                { value: 'en', name: 'English' },
                { value: 'ja', name: '日本語' },
              ]"
            />
            <select-wrapper
              v-model="editingSettings.global.appearance.darkMode"
              class="w-80"
              :label="t('dialog.settings.global.appearance.darkMode')"
              :items="[
                { value: 'system', name: t('dialog.settings.global.appearance.systemSettings') },
                { value: 'dark', name: 'Dark' },
                { value: 'light', name: 'Light' },
              ]"
            />
            <checkbox-wrapper
              v-model="editingSettings.global.appearance.hideControls"
              :label="t('dialog.settings.global.appearance.hideControls')"
            />
          </div>
          <div
            v-if="'audio' in editingSettings.global"
            v-show="tab === 'audioHardware'"
            class="flex flex-col gap-4 p-4"
          >
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
            <select-wrapper
              v-model="editingSettings.global.audio.deviceId"
              :label="t('dialog.settings.global.audioHardware.device')"
              :items="devices"
            />
            <select-wrapper
              v-model="editingSettings.global.audio.channelCount"
              :label="t('dialog.settings.global.audioHardware.channelCount')"
              :items="channelCounts"
            />
            <select-wrapper
              v-model="editingSettings.global.audio.sampleRate"
              :label="t('dialog.settings.global.audioHardware.sampleRate')"
              :items="sampleRates"
            />
            <select-wrapper
              v-model="editingSettings.global.audio.bufferSize"
              :label="t('dialog.settings.global.audioHardware.bufferSize')"
              :items="bufferSizes"
            />
          </div>
          <div
            v-show="tab === 'hotkey'"
            class="p-3"
          >
            <h2 class="mb-3">
              {{ t('dialog.settings.global.hotkey.playback.title') }}
            </h2>
            <div class="flex flex-row items-start gap-4">
              <div class="flex flex-col gap-4">
                <hotkey-input
                  v-model="editingSettings.global.hotkey.playback.go"
                  class="w-70"
                  :label="t('dialog.settings.global.hotkey.playback.go')"
                />
                <hotkey-input
                  v-model="editingSettings.global.hotkey.playback.load"
                  class="w-70"
                  :label="t('dialog.settings.global.hotkey.playback.load')"
                />
                <hotkey-input
                  v-model="editingSettings.global.hotkey.playback.pauseAndResume"
                  class="w-70"
                  :label="t('dialog.settings.global.hotkey.playback.pauseAndResume')"
                />
                <hotkey-input
                  v-model="editingSettings.global.hotkey.playback.pauseAll"
                  class="w-70"
                  :label="t('dialog.settings.global.hotkey.playback.pauseAll')"
                />
                <hotkey-input
                  v-model="editingSettings.global.hotkey.playback.resumeAll"
                  class="w-70"
                  :label="t('dialog.settings.global.hotkey.playback.resumeAll')"
                />
              </div>
              <div class="flex flex-col gap-4">
                <hotkey-input
                  v-model="editingSettings.global.hotkey.playback.stop"
                  class="w-70"
                  :label="t('dialog.settings.global.hotkey.playback.stop')"
                />
                <hotkey-input
                  v-model="editingSettings.global.hotkey.playback.stopAll"
                  class="w-70"
                  :label="t('dialog.settings.global.hotkey.playback.stopAll')"
                />
                <hotkey-input
                  v-model="editingSettings.global.hotkey.playback.seekForward"
                  class="w-70"
                  :label="t('dialog.settings.global.hotkey.playback.seekForward')"
                />
                <hotkey-input
                  v-model="editingSettings.global.hotkey.playback.seekBackward"
                  class="w-70"
                  :label="t('dialog.settings.global.hotkey.playback.seekBackward')"
                />
              </div>
            </div>
            <divider />
            <h2 class="my-3">
              {{ t('dialog.settings.global.hotkey.audio.title') }}
            </h2>
            <div class="flex flex-col gap-4">
              <hotkey-input
                v-model="editingSettings.global.hotkey.audioAction.toggleRepeat"
                class="w-70"
                :label="t('dialog.settings.global.hotkey.audio.toggleRepeat')"
              />
            </div>
          </div>
          <div
            v-show="tab === 'template'"
            class="h-full"
          >
            <template-settings v-model="editingSettings" />
          </div>
          <div
            v-show="tab === 'nameFormat'"
            class="h-full"
          >
            <div class="flex flex-col gap-5 p-4">
              <text-input
                v-model="editingSettings.global.nameFormat.audio"
                class="w-125"
                :label="t('dialog.settings.global.nameFormat.audio.title')"
                :help="t('dialog.settings.global.nameFormat.audio.description')"
              />
              <text-input
                v-model="editingSettings.global.nameFormat.wait"
                class="w-125"
                :label="t('dialog.settings.global.nameFormat.wait.title')"
                :help="t('dialog.settings.global.nameFormat.wait.description')"
              />
              <text-input
                v-model="editingSettings.global.nameFormat.fade"
                class="w-125"
                :label="t('dialog.settings.global.nameFormat.fade.title')"
                :help="t('dialog.settings.global.nameFormat.fade.description')"
              />
              <text-input
                v-model="editingSettings.global.nameFormat.start"
                class="w-125"
                :label="t('dialog.settings.global.nameFormat.start.title')"
                :help="t('dialog.settings.global.nameFormat.playbackDescription')"
              />
              <text-input
                v-model="editingSettings.global.nameFormat.stop"
                class="w-125"
                :label="t('dialog.settings.global.nameFormat.stop.title')"
                :help="t('dialog.settings.global.nameFormat.playbackDescription')"
              />
              <text-input
                v-model="editingSettings.global.nameFormat.pause"
                class="w-125"
                :label="t('dialog.settings.global.nameFormat.pause.title')"
                :help="t('dialog.settings.global.nameFormat.playbackDescription')"
              />
              <text-input
                v-model="editingSettings.global.nameFormat.load"
                class="w-125"
                :label="t('dialog.settings.global.nameFormat.load.title')"
                :help="t('dialog.settings.global.nameFormat.playbackDescription')"
              />
              <text-input
                v-model="editingSettings.global.nameFormat.group"
                class="w-125"
                :label="t('dialog.settings.global.nameFormat.group.title')"
                :help="'{mode} replaced with Group cue mode'"
              />
            </div>
          </div>
        </div>
      </div>
      <div class="mr-0 ml-0 flex w-full grow-0 items-center gap-4">
        <button-wrapper
          class="ml-auto"
          :label="t('general.cancel')"
          severity="secondary"
          @click="isSettingsDialogOpen = false"
        />
        <button-wrapper
          severity="primary"
          :label="t('general.done')"
          @click="
            saveSettings().then((value) => {
              if (value) isSettingsDialogOpen = false;
            })
          "
        />
      </div>
    </div>
  </Drawer>
</template>

<style lang="css" module>
.selected-category {
  background-color: rgb(from var(--p-primary-color) r g b / 0.2);
}
</style>
