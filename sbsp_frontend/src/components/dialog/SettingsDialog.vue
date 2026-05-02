<template>
  <v-dialog
    v-model="isSettingsDialogOpen"
    width="auto"
    fullscreen
    @keydown.esc.stop="isSettingsDialogOpen = false"
    @keydown.stop
    @contextmenu.prevent
  >
    <v-sheet class="d-flex flex-column w-100 h-100">
      <v-sheet class="flex-grow-1 d-flex flex-row w-100">
        <v-tabs
          v-model="tab"
          direction="vertical"
        >
          <v-tab
            :text="t('dialog.settings.tab.preset')"
            value="preset"
          />
          <v-sheet class="pa-1 text-caption">
            {{ t('dialog.settings.tab.category.inThisShowModel') }}
          </v-sheet>
          <v-tab
            :text="t('dialog.settings.tab.general')"
            value="showGeneral"
          />
          <v-tab
            :text="t('dialog.settings.tab.audioLogic')"
            value="audioLogic"
          />
          <v-tab
            :text="t('dialog.settings.tab.remote')"
            value="remote"
          />
          <v-sheet class="pa-1 text-caption">
            {{ t('dialog.settings.tab.category.global') }}
          </v-sheet>
          <v-tab
            :text="t('dialog.settings.tab.general')"
            value="globalGeneral"
          />
          <v-tab
            :text="t('dialog.settings.tab.appearance')"
            value="appearance"
          />
          <v-tab
            v-if="'audio' in editingSettings.global"
            :text="t('dialog.settings.tab.audioHardware')"
            value="audioHardware"
          />
          <v-tab
            :text="t('dialog.settings.tab.hotkey')"
            value="hotkey"
          />
          <v-tab
            :text="t('dialog.settings.tab.template')"
            value="template"
          />
          <v-tab
            :text="t('dialog.settings.tab.nameFormat')"
            value="nameFormat"
          />
        </v-tabs>
        <v-divider
          vertical
          opacity="0.5"
        />
        <v-tabs-window
          v-model="tab"
          class="flex-grow-1 fill-height"
        >
          <v-tabs-window-item
            value="preset"
            class="pa-4"
          >
            <h2 class="mb-4">
              {{ t('dialog.settings.preset.hotkeyCursor.title') }}
            </h2>
            <v-sheet class="d-flex flex-row ga-3 align-center">
              <v-btn
                class="text-none"
                width="120px"
                variant="flat"
                color="primary"
                @click="recallMusicBeePreset"
              >
                MusicBee
              </v-btn>
              <span>{{ t('dialog.settings.preset.hotkeyCursor.musicBee.description') }}</span>
            </v-sheet>
            <v-divider class="mt-4 mb-4" />
            <v-sheet class="d-flex flex-row ga-3 align-center">
              <v-btn
                class="text-none"
                width="120px"
                variant="flat"
                color="primary"
                @click="recallQLabPreset"
              >
                QLab
              </v-btn>
              <span>{{ t('dialog.settings.preset.hotkeyCursor.qLab.description') }}</span>
            </v-sheet>
          </v-tabs-window-item>
          <v-tabs-window-item
            value="showGeneral"
            class="pa-3"
          >
            <text-input
              v-model="showModelName"
              class="mt-4"
              align-input="left"
              width="500px"
              :label="t('dialog.settings.show.general.showModelName')"
            />
            <text-input
              v-model="editingSettings.show.general.copyAssetsDestination"
              align-input="left"
              class="mt-4"
              width="500px"
              :label="t('dialog.settings.show.general.assetsDirectory.title')"
              :hint="t('dialog.settings.show.general.assetsDirectory.description')"
              persistent-hint
              show-details
            />
          </v-tabs-window-item>
          <v-tabs-window-item
            value="audioLogic"
            class="pa-4"
          >
            <v-checkbox
              v-model="editingSettings.show.audio.monoOutput"
              :label="t('dialog.settings.show.audioLogic.monoOutput')"
              hide-details
            />
            <v-number-input
              v-model="editingSettings.show.audio.lufsTarget"
              class="mt-4"
              :label="t('dialog.settings.show.audioLogic.targetLufs')"
              suffix="LUFS"
              density="compact"
              variant="outlined"
              hide-details
              width="200px"
              autocomplete="off"
              :precision="2"
            />
          </v-tabs-window-item>
          <v-tabs-window-item
            value="remote"
            class="pa-3"
          >
            <v-checkbox
              v-model="editingSettings.show.remote.lockCursorToSelection"
              :label="t('dialog.settings.show.remote.lockCursorToSelection')"
            />
          </v-tabs-window-item>
          <v-tabs-window-item value="globalGeneral">
            <v-checkbox
              v-model="editingSettings.global.general.advanceCursorWhenGo"
              :label="t('dialog.settings.global.general.advanceCursorWhenGo')"
              hide-details
            />
            <v-divider />
            <v-checkbox
              v-model="editingSettings.global.general.lockCursorToSelection"
              :label="t('dialog.settings.global.general.lockCursorToSelection')"
              hide-details
            />
            <v-divider />
            <v-checkbox
              v-model="editingSettings.global.general.copyAssetsWhenAdd"
              :label="t('dialog.settings.global.general.copyAssetsWhenAdd')"
              hide-details
            />
            <v-divider />
            <v-number-input
              v-model="editingSettings.global.general.seekAmount"
              hide-details
              inset
              persistent-placeholder
              class="ma-3"
              :min="0"
              width="160px"
              :label="t('dialog.settings.global.general.seekAmount')"
              density="compact"
              variant="outlined"
              autocomplete="off"
              @keydown.stop
              :precision="2"
            />
          </v-tabs-window-item>
          <v-tabs-window-item
            value="appearance"
            class="pa-4"
          >
            <v-sheet class="d-flex flex-column ga-4">
              <v-select
                v-model="editingSettings.global.appearance.language"
                hide-details
                persistent-placeholder
                :label="t('dialog.settings.global.appearance.language')"
                :items="[
                  { value: null, name: t('dialog.settings.global.appearance.systemLanguage') },
                  { value: 'en', name: 'English' },
                  { value: 'ja', name: '日本語' },
                ]"
                item-value="value"
                item-title="name"
                variant="outlined"
                density="compact"
                autocomplete="off"
                @keydown.stop
              />
              <v-select
                v-model="editingSettings.global.appearance.darkMode"
                hide-details
                persistent-placeholder
                :label="t('dialog.settings.global.appearance.darkMode')"
                :items="[
                  { value: 'system', name: t('dialog.settings.global.appearance.systemSettings') },
                  { value: 'dark', name: 'Dark' },
                  { value: 'light', name: 'Light' },
                ]"
                item-value="value"
                item-title="name"
                variant="outlined"
                density="compact"
                autocomplete="off"
                @keydown.stop
              />
              <v-checkbox
                v-model="editingSettings.global.appearance.hideControls"
                :label="t('dialog.settings.global.appearance.hideControls')"
                hide-details
              />
            </v-sheet>
          </v-tabs-window-item>
          <v-tabs-window-item
            v-if="'audio' in editingSettings.global"
            value="audioHardware"
            class="d-flex flex-column pa-3 ga-4"
          >
            <div>
              <v-alert
                class="flex-shrink-0"
                type="error"
                icon="$warning"
                :text="t('dialog.settings.global.audioHardware.warning')"
              />
            </div>
            <v-select
              v-model="editingSettings.global.audio.deviceId"
              hide-details
              persistent-placeholder
              :label="t('dialog.settings.global.audioHardware.device')"
              :items="devices"
              item-value="value"
              item-title="name"
              variant="outlined"
              density="compact"
              autocomplete="off"
              @keydown.stop
            />
            <v-select
              v-model="editingSettings.global.audio.channelCount"
              hide-details
              persistent-placeholder
              :label="t('dialog.settings.global.audioHardware.channelCount')"
              :items="channelCounts"
              item-value="value"
              item-title="name"
              variant="outlined"
              density="compact"
              autocomplete="off"
              @keydown.stop
            />
            <v-select
              v-model="editingSettings.global.audio.sampleRate"
              hide-details
              persistent-placeholder
              :label="t('dialog.settings.global.audioHardware.sampleRate')"
              :items="sampleRates"
              item-value="value"
              item-title="name"
              variant="outlined"
              density="compact"
              autocomplete="off"
              @keydown.stop
            />
            <v-select
              v-model="editingSettings.global.audio.bufferSize"
              hide-details
              persistent-placeholder
              :label="t('dialog.settings.global.audioHardware.bufferSize')"
              :items="bufferSizes"
              item-value="value"
              item-title="name"
              variant="outlined"
              density="compact"
              autocomplete="off"
              @keydown.stop
            />
          </v-tabs-window-item>
          <v-tabs-window-item
            value="hotkey"
            class="pa-3"
          >
            <h2 class="mb-3">
              {{ t('dialog.settings.global.hotkey.playback.title') }}
            </h2>
            <div class="d-flex flex-row ga-4 px-3 align-start">
              <div class="d-flex flex-column">
                <hotkey-input
                  v-model="editingSettings.global.hotkey.playback.go"
                  width="280px"
                  :label="t('dialog.settings.global.hotkey.playback.go')"
                />
                <hotkey-input
                  v-model="editingSettings.global.hotkey.playback.load"
                  width="280px"
                  :label="t('dialog.settings.global.hotkey.playback.load')"
                />
                <hotkey-input
                  v-model="editingSettings.global.hotkey.playback.pauseAndResume"
                  width="280px"
                  :label="t('dialog.settings.global.hotkey.playback.pauseAndResume')"
                />
                <hotkey-input
                  v-model="editingSettings.global.hotkey.playback.pauseAll"
                  width="280px"
                  :label="t('dialog.settings.global.hotkey.playback.pauseAll')"
                />
                <hotkey-input
                  v-model="editingSettings.global.hotkey.playback.resumeAll"
                  width="280px"
                  :label="t('dialog.settings.global.hotkey.playback.resumeAll')"
                />
              </div>
              <div class="d-flex flex-column">
                <hotkey-input
                  v-model="editingSettings.global.hotkey.playback.stop"
                  width="280px"
                  :label="t('dialog.settings.global.hotkey.playback.stop')"
                />
                <hotkey-input
                  v-model="editingSettings.global.hotkey.playback.stopAll"
                  width="280px"
                  :label="t('dialog.settings.global.hotkey.playback.stopAll')"
                />
                <hotkey-input
                  v-model="editingSettings.global.hotkey.playback.seekForward"
                  width="280px"
                  :label="t('dialog.settings.global.hotkey.playback.seekForward')"
                />
                <hotkey-input
                  v-model="editingSettings.global.hotkey.playback.seekBackward"
                  width="280px"
                  :label="t('dialog.settings.global.hotkey.playback.seekBackward')"
                />
              </div>
            </div>
            <v-divider />
            <h2 class="my-3">
              {{ t('dialog.settings.global.hotkey.audio.title') }}
            </h2>
            <div class="px-3">
              <hotkey-input
                v-model="editingSettings.global.hotkey.audioAction.toggleRepeat"
                width="280px"
                :label="t('dialog.settings.global.hotkey.audio.toggleRepeat')"
              />
            </div>
          </v-tabs-window-item>
          <v-tabs-window-item
            value="template"
            class="fill-height"
          >
            <template-settings v-model="editingSettings" />
          </v-tabs-window-item>
          <v-tabs-window-item
            value="nameFormat"
            class="fill-height"
          >
            <v-sheet
              flat
              class="d-flex flex-column pa-4 ga-4"
            >
              <text-input
                v-model="editingSettings.global.nameFormat.audio"
                align-input="left"
                class="mt-4"
                width="500px"
                :label="t('dialog.settings.global.nameFormat.audio.title')"
                :hint="t('dialog.settings.global.nameFormat.audio.description')"
                persistent-hint
                show-details
              />
              <text-input
                v-model="editingSettings.global.nameFormat.wait"
                align-input="left"
                class="mt-4"
                width="500px"
                :label="t('dialog.settings.global.nameFormat.wait.title')"
                :hint="t('dialog.settings.global.nameFormat.wait.description')"
                persistent-hint
                show-details
              />
              <text-input
                v-model="editingSettings.global.nameFormat.fade"
                align-input="left"
                class="mt-4"
                width="500px"
                :label="t('dialog.settings.global.nameFormat.fade.title')"
                :hint="t('dialog.settings.global.nameFormat.fade.description')"
                persistent-hint
                show-details
              />
              <text-input
                v-model="editingSettings.global.nameFormat.start"
                align-input="left"
                class="mt-4"
                width="500px"
                :label="t('dialog.settings.global.nameFormat.start.title')"
                :hint="t('dialog.settings.global.nameFormat.playbackDescription')"
                persistent-hint
                show-details
              />
              <text-input
                v-model="editingSettings.global.nameFormat.stop"
                align-input="left"
                class="mt-4"
                width="500px"
                :label="t('dialog.settings.global.nameFormat.stop.title')"
                :hint="t('dialog.settings.global.nameFormat.playbackDescription')"
                persistent-hint
                show-details
              />
              <text-input
                v-model="editingSettings.global.nameFormat.pause"
                align-input="left"
                class="mt-4"
                width="500px"
                :label="t('dialog.settings.global.nameFormat.pause.title')"
                :hint="t('dialog.settings.global.nameFormat.playbackDescription')"
                persistent-hint
                show-details
              />
              <text-input
                v-model="editingSettings.global.nameFormat.load"
                align-input="left"
                class="mt-4"
                width="500px"
                :label="t('dialog.settings.global.nameFormat.load.title')"
                :hint="t('dialog.settings.global.nameFormat.playbackDescription')"
                persistent-hint
                show-details
              />
              <text-input
                v-model="editingSettings.global.nameFormat.group"
                align-input="left"
                class="mt-4"
                width="500px"
                :label="t('dialog.settings.global.nameFormat.group.title')"
              />
            </v-sheet>
          </v-tabs-window-item>
        </v-tabs-window>
      </v-sheet>
      <v-divider
        thickness="1"
        opacity="0.5"
      />
      <v-footer class="flex-grow-0 d-flex align-center ml-0 mr-0 w-100">
        <v-btn
          class="ml-auto"
          :text="t('general.cancel')"
          @click="isSettingsDialogOpen = false"
        />
        <v-btn
          color="primary"
          :text="t('general.done')"
          @click="
            saveSettings().then(value => {
              if (value) isSettingsDialogOpen = false;
            });
          "
        />
      </v-footer>
    </v-sheet>
  </v-dialog>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, toRaw, watch } from 'vue';
import { useShowModel } from '../../stores/showmodel';
import type { ShowSettings } from '../../types/ShowSettings';
import HotkeyInput from '../input/HotkeyInput.vue';
import TextInput from '../input/TextInput.vue';
import { useUiSettings } from '../../stores/uiSettings';
import type { GlobalHostSettings } from '../../types/GlobalHostSettings';
import type { GlobalRemoteSettings } from '../../types/GlobalRemoteSettings';
import { useI18n } from 'vue-i18n';
import { side, useApi } from '../../api';
import TemplateSettings from './settings/TemplateSettings.vue';
import { SupportedHardware } from '../../types/SupportedHardware';
import { useShowState } from '../../stores/showstate';
import { message } from '@tauri-apps/plugin-dialog';
import { AudioHardwareSettings } from '../../types/AudioHardwareSettings';

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

const devices = computed(() => {
  const supportedHW = supportedHardware.value;
  if (supportedHW != null) {
    let devices: { name: string; value: string | null }[] = [{ name: t('general.default'), value: null }];
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
      const channels: { name: string; value: number | null }[] = [{ name: `${t('general.default')} (${device.defaultChannelCount})`, value: null }];
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
      const channels = editingSettings.value.global.audio.channelCount || device.defaultChannelCount;
      let sampleRates: { name: string; value: number | null }[] = [{ name: `${t('general.default')} (${device.defaultSampleRate / 1000} kHz)`, value: null }];
      for (const fc of device.supportedConfigs) {
        if (fc.channelCount == channels) {
          sampleRates = sampleRates.concat(fc.sampleRates.map(sr => ({ value: sr, name: (sr / 1000).toString() + ' kHz' })));
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
      const channels = editingSettings.value.global.audio.channelCount || device.defaultChannelCount;
      let bufferSizes: { name: string; value: number | null }[] = [{ name: `${t('general.default')}`, value: null }];
      for (const fc of device.supportedConfigs) {
        if (fc.channelCount == channels) {
          bufferSizes = bufferSizes.concat(fc.bufferSizes.map(bs => ({ value: bs, name: bs.toString() + ' Frames' })));
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
    api.host.getHardware().then(value => supportedHardware.value = value);
  }
});

const saveSettings = async (): Promise<boolean> => {
  if (side == 'host' && 'audio' in uiSettings.settings && 'audio' in editingSettings.value.global && !isEqualAudioHardware(editingSettings.value.global.audio, uiSettings.settings.audio)) {
    const activeIds = Object.keys(showState.activeCues);
    let hasActiveAudioCue = false;
    for (const id of activeIds) {
      if (showModel.getCueById(id)?.params.type == 'audio') {
        hasActiveAudioCue = true;
        break;
      };
    }
    if (hasActiveAudioCue) {
      const result = await message(t('dialog.settings.global.audioHardware.saveWarning'), {
        buttons: 'OkCancel',
        kind: 'warning',
        title: t('general.warning'),
      });
      if (result) {
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
  return a.deviceId == b.deviceId && a.channelCount == b.channelCount && a.sampleRate == b.sampleRate && a.bufferSize == b.bufferSize;
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
