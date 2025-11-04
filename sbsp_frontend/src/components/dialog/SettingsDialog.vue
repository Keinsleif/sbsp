<template>
  <v-dialog
    v-model="isSettingsDialogOpen"
    width="auto"
    fullscreen
    @keydown.esc.stop="isSettingsDialogOpen = false"
    @keydown.stop
  >
    <v-sheet class="d-flex flex-column w-100 h-100">
      <v-sheet class="flex-grow-1 d-flex flex-row w-100">
        <v-tabs v-model="tab" direction="vertical">
          <v-tab text="Preset" value="preset"></v-tab>
          <v-sheet class="pa-1 text-caption">ShowModel</v-sheet>
          <v-tab text="General" value="showGeneral"></v-tab>
          <v-tab text="Audio" value="audio"></v-tab>
          <v-tab text="Remote" value="remote"></v-tab>
          <v-sheet class="pa-1 text-caption">Global</v-sheet>
          <v-tab text="General" value="globalGeneral"></v-tab>
          <v-tab text="Hotkey" value="hotkey"></v-tab>
          <v-tab text="Template" value="template"></v-tab>
        </v-tabs>
        <v-divider vertical opacity="0.5"></v-divider>
        <v-tabs-window v-model="tab" class="flex-grow-1 fill-height">
          <v-tabs-window-item value="preset" class="pa-4">
            <h2 class="mb-4">Hotkey & Cursor behavier</h2>
            <v-sheet class="d-flex flex-row ga-3 align-center">
              <v-btn class="text-none" width="120px" variant="flat" color="primary" @click="recallMusicBeePreset"
                >MusicBee</v-btn
              >
              <span>Preset for MusicBee like controls.</span>
            </v-sheet>
            <v-divider class="mt-4 mb-4"></v-divider>
            <v-sheet class="d-flex flex-row ga-3 align-center">
              <v-btn class="text-none" width="120px" variant="flat" color="primary" @click="recallQLabPreset"
                >QLab</v-btn
              >
              <span>Preset for QLab like controls.</span>
            </v-sheet>
          </v-tabs-window-item>
          <v-tabs-window-item value="showGeneral" class="pa-3">
            <text-input
              v-model="showModelName"
              class="mt-4"
              align-input="left"
              width="500px"
              label="Show Model name"
            ></text-input>
            <text-input
              v-model="editingSettings.show.general.copyAssetsDestination"
              align-input="left"
              class="mt-4"
              width="500px"
              label="Assets directory"
              hint="Relative path to the directory for copying assets. (per-show setting)"
              persistent-hint
              show-details
            ></text-input>
          </v-tabs-window-item>
          <v-tabs-window-item value="audio" class="pa-3">
            <v-checkbox v-model="editingSettings.show.audio.monoOutput" label="Downmix stereo to mono"></v-checkbox>
          </v-tabs-window-item>
          <v-tabs-window-item value="remote" class="pa-3">
            <v-checkbox
              v-model="editingSettings.show.remote.lockCursorToSelection"
              label="Lock Cursor to Selection (on Remote side)"
            ></v-checkbox>
          </v-tabs-window-item>
          <v-tabs-window-item value="globalGeneral">
            <v-checkbox
              v-model="editingSettings.global.general.advanceCursorWhenGo"
              label="Advance cursor when GO"
              hide-details
            ></v-checkbox>
            <v-divider></v-divider>
            <v-checkbox
              v-model="editingSettings.global.general.lockCursorToSelection"
              label="Lock Cursor to Selection (on Main side)"
              hide-details
            ></v-checkbox>
            <v-divider></v-divider>
            <v-checkbox
              v-model="editingSettings.global.general.copyAssetsWhenAdd"
              label="Copy assets when adding Cue"
              hide-details
            ></v-checkbox>
            <v-divider></v-divider>
            <v-number-input
              hide-details
              inset
              persistent-placeholder
              class="ma-3"
              v-model="editingSettings.global.general.seekAmount"
              :min="0"
              width="160px"
              label="Seek amount"
              density="compact"
              variant="outlined"
              autocomplete="off"
              @keydown.stop
            ></v-number-input>
          </v-tabs-window-item>
          <v-tabs-window-item value="hotkey" class="pa-3">
            <h2 class="mb-3">Playback</h2>
            <hotkey-input v-model="editingSettings.global.hotkey.playback.go" width="280px" label="Go"></hotkey-input>
            <hotkey-input
              v-model="editingSettings.global.hotkey.playback.load"
              width="280px"
              label="Load"
            ></hotkey-input>
            <hotkey-input
              v-model="editingSettings.global.hotkey.playback.pauseAndResume"
              width="280px"
              label="Pause & Resume"
            ></hotkey-input>
            <hotkey-input
              v-model="editingSettings.global.hotkey.playback.pauseAll"
              width="280px"
              label="Pause All"
            ></hotkey-input>
            <hotkey-input
              v-model="editingSettings.global.hotkey.playback.resumeAll"
              width="280px"
              label="Resume All"
            ></hotkey-input>
            <hotkey-input
              v-model="editingSettings.global.hotkey.playback.stop"
              width="280px"
              label="Stop"
            ></hotkey-input>
            <hotkey-input
              v-model="editingSettings.global.hotkey.playback.stopAll"
              width="280px"
              label="Stop All"
            ></hotkey-input>
            <hotkey-input
              v-model="editingSettings.global.hotkey.playback.seekForward"
              width="280px"
              label="Seek Forward"
            ></hotkey-input>
            <hotkey-input
              v-model="editingSettings.global.hotkey.playback.seekBackward"
              width="280px"
              label="Seek Backward"
            ></hotkey-input>
            <v-divider></v-divider>
            <h2 class="mb-3 mt-3">Audio Action</h2>
            <hotkey-input
              v-model="editingSettings.global.hotkey.audioAction.toggleRepeat"
              width="280px"
              label="ToggleRepeat"
            ></hotkey-input>
          </v-tabs-window-item>
          <v-tabs-window-item value="template" class="fill-height">
            <v-sheet class="d-flex flex-column w-100" height="100%">
              <v-table fixed-header density="compact" class="flex-grow-1" height="100%">
                <thead>
                  <tr>
                    <th id="cuelist_type">Type</th>
                    <th id="cuelist_number">Number</th>
                    <th id="cuelist_name">Name</th>
                    <th id="cuelist_pre_wait" class="text-center">Pre-Wait</th>
                    <th id="cuelist_duration" class="text-center">Duration</th>
                    <th id="cuelist_post_wait" class="text-center">Post-Wait</th>
                    <th id="cuelist_repeat"><v-icon :icon="mdiRepeat"></v-icon></th>
                    <th id="cuelist_sequence"><v-icon :icon="mdiChevronDoubleDown" /></th>
                  </tr>
                </thead>
                <tbody>
                  <tr
                    :class="[selectingTemplate == 'audio' ? $style['selected-row'] : '']"
                    @mousedown="selectingTemplate = 'audio'"
                  >
                    <td headers="cuelist_type" width="160px">Audio</td>
                    <td headers="cuelist_number" class="text-center" width="50px">
                      {{ editingSettings.global.template.audio.number }}
                    </td>
                    <td headers="cuelist_name" width="auto">
                      {{
                        editingSettings.global.template.audio.name != null
                          ? editingSettings.global.template.audio.name
                          : '** built from cue param **'
                      }}
                    </td>
                    <td headers="cuelist_pre_wait" class="text-center pa-1" width="100px">
                      <div>
                        {{ secondsToFormat(editingSettings.global.template.audio.preWait) }}
                      </div>
                    </td>
                    <td headers="cuelist_duration" class="text-center pa-1" width="100px">
                      <div>
                        {{ secondsToFormat(calculateDuration(editingSettings.global.template.audio.params, null)) }}
                      </div>
                    </td>
                    <td headers="cuelist_post_wait" class="text-center pa-1" width="100px">
                      <div>
                        {{
                          editingSettings.global.template.audio.sequence.type == 'doNotContinue'
                            ? '--:--.--'
                            : editingSettings.global.template.audio.sequence.type == 'autoContinue'
                              ? secondsToFormat(editingSettings.global.template.audio.sequence.postWait)
                              : secondsToFormat(calculateDuration(editingSettings.global.template.audio.params, null))
                        }}
                      </div>
                    </td>
                    <td headers="cuelist_repeat">
                      <v-icon
                        v-if="
                          editingSettings.global.template.audio.params.type == 'audio' &&
                          editingSettings.global.template.audio.params.repeat
                        "
                        :icon="mdiRepeat"
                      />
                    </td>
                    <td headers="cuelist_sequence">
                      <v-icon
                        v-if="editingSettings.global.template.audio.sequence.type == 'autoFollow'"
                        :icon="mdiArrowExpandDown"
                      />
                      <v-icon
                        v-if="editingSettings.global.template.audio.sequence.type == 'autoContinue'"
                        :icon="mdiArrowDown"
                      />
                    </td>
                  </tr>
                  <tr
                    :class="[selectingTemplate == 'wait' ? $style['selected-row'] : '']"
                    @mousedown="selectingTemplate = 'wait'"
                  >
                    <td headers="cuelist_type" width="160px">Wait</td>
                    <td headers="cuelist_number" class="text-center" width="50px">
                      {{ editingSettings.global.template.wait.number }}
                    </td>
                    <td headers="cuelist_name" width="auto">
                      {{
                        editingSettings.global.template.wait.name != null
                          ? editingSettings.global.template.wait.name
                          : '** built from cue param **'
                      }}
                    </td>
                    <td headers="cuelist_pre_wait" class="text-center pa-1" width="100px">
                      <div>
                        {{ secondsToFormat(editingSettings.global.template.wait.preWait) }}
                      </div>
                    </td>
                    <td headers="cuelist_duration" class="text-center pa-1" width="100px">
                      <div>
                        {{
                          secondsToFormat(
                            editingSettings.global.template.wait.params.type == 'wait'
                              ? editingSettings.global.template.wait.params.duration
                              : null,
                          )
                        }}
                      </div>
                    </td>
                    <td headers="cuelist_post_wait" class="text-center pa-1" width="100px">
                      <div>
                        {{
                          editingSettings.global.template.wait.sequence.type == 'doNotContinue'
                            ? '--:--.--'
                            : editingSettings.global.template.wait.sequence.type == 'autoContinue'
                              ? secondsToFormat(editingSettings.global.template.wait.sequence.postWait)
                              : secondsToFormat(calculateDuration(editingSettings.global.template.wait.params, null))
                        }}
                      </div>
                    </td>
                    <td headers="cuelist_repeat">
                      <v-icon
                        v-if="
                          editingSettings.global.template.wait.params.type == 'audio' &&
                          editingSettings.global.template.wait.params.repeat
                        "
                        :icon="mdiRepeat"
                      />
                    </td>
                    <td headers="cuelist_sequence">
                      <v-icon
                        v-if="editingSettings.global.template.wait.sequence.type == 'autoFollow'"
                        :icon="mdiArrowExpandDown"
                      />
                      <v-icon
                        v-if="editingSettings.global.template.wait.sequence.type == 'autoContinue'"
                        :icon="mdiArrowDown"
                      />
                    </td>
                  </tr>
                </tbody>
              </v-table>
              <div style="height: 302px" class="flex-grow-0 mb-0">
                <bottom-editor v-model="selectingCue"></bottom-editor>
              </div>
            </v-sheet>
          </v-tabs-window-item>
        </v-tabs-window>
      </v-sheet>
      <v-divider thickness="1" opacity="0.5"></v-divider>
      <v-footer class="flex-grow-0 d-flex align-center ml-0 mr-0 w-100">
        <v-btn class="ml-auto" text="Cancel" @click="isSettingsDialogOpen = false"></v-btn>
        <v-btn
          text="Done"
          @click="
            saveSettings();
            isSettingsDialogOpen = false;
          "
        ></v-btn>
      </v-footer>
    </v-sheet>
  </v-dialog>
</template>

<script setup lang="ts">
import { ref, toRaw, watch } from 'vue';
import { useShowModel } from '../../stores/showmodel';
import { invoke } from '@tauri-apps/api/core';
import type { ShowSettings } from '../../types/ShowSettings';
import HotkeyInput from '../input/HotkeyInput.vue';
import { mdiChevronDoubleDown, mdiRepeat, mdiArrowExpandDown, mdiArrowDown } from '@mdi/js';
import { secondsToFormat, calculateDuration } from '../../utils';
import BottomEditor from '../BottomEditor.vue';
import type { Cue } from '../../types/Cue';
import TextInput from '../input/TextInput.vue';
import { useUiSettings } from '../../stores/uiSettings';
import type { GlobalSettings } from '../../types/GlobalSettings';

const showModel = useShowModel();
const uiSettings = useUiSettings();

const isSettingsDialogOpen = defineModel<boolean>({ required: true });

const tab = ref('showGeneral');
const selectingTemplate = ref<'audio' | 'wait' | null>(null);
const showModelName = ref<string>(showModel.name);
const editingSettings = ref<{
  show: ShowSettings;
  global: GlobalSettings;
}>({ show: structuredClone(toRaw(showModel.settings)), global: structuredClone(toRaw(uiSettings.settings)) });

const getSelectingCue = () => {
  if (selectingTemplate.value == 'audio') {
    return editingSettings.value.global.template.audio;
  } else if (selectingTemplate.value == 'wait') {
    return editingSettings.value.global.template.wait;
  }
  return null;
};

const selectingCue = ref<Cue | null>(getSelectingCue());

watch(
  () => selectingTemplate.value,
  () => {
    selectingCue.value = getSelectingCue();
  },
);

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
  (newSettings) => {
    editingSettings.value.global = structuredClone(toRaw(newSettings));
  },
);

watch(isSettingsDialogOpen, (newState) => {
  if (newState) {
    editingSettings.value = {
      show: structuredClone(toRaw(showModel.settings)),
      global: structuredClone(toRaw(uiSettings.settings)),
    };
  }
});

const saveSettings = () => {
  invoke('update_show_settings', { newSettings: editingSettings.value.show }).catch((e) => console.error(e));
  invoke('update_model_name', { newName: showModelName.value }).catch((e) => console.error(e));
  uiSettings.update(editingSettings.value.global);
  uiSettings.save();
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

<style lang="css" module>
.selected-row > td {
  background-color: rgb(var(--v-theme-primary), 0.2);
  color: rgb(var(--v-theme-on-background));
}
</style>
