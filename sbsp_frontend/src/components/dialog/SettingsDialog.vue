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
          <v-tab text="General" value="general"></v-tab>
          <v-tab text="Hotkey" value="hotkey"></v-tab>
          <v-tab text="Template" value="template"></v-tab>
          <v-tab text="Audio" value="audio"></v-tab>
          <v-tab text="Remote" value="remote"></v-tab>
        </v-tabs>
        <v-divider vertical opacity="0.5"></v-divider>
        <v-tabs-window v-model="tab" class="flex-grow-1 fill-height">
          <v-tabs-window-item
            value="general"
            transition="toggle-slide-y-transition"
            reverse-transition="toggle-slide-y-reverse-transition"
          >
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
              class="ma-2"
              v-model="editingSettings.global.general.seekAmount"
              :min="0"
              width="160px"
              label="Seek amount"
              density="compact"
              variant="outlined"
              autocomplete="off"
              @keydown.stop
            ></v-number-input>
            <v-divider></v-divider>
            <text-input
              v-model="editingSettings.show.general.copyAssetsDestination"
              class="ma-2"
              align-input="left"
              width="500px"
              label="Assets directory"
              hint="Relative path to the directory for copying assets. (per-show setting)"
              persistent-hint
              show-details
            ></text-input>
          </v-tabs-window-item>
          <v-tabs-window-item
            value="hotkey"
            class="pa-3"
            transition="toggle-slide-y-transition"
            reverse-transition="toggle-slide-y-reverse-transition"
          >
            <h2>Playback</h2>
            <hotkey-input v-model="editingSettings.global.hotkey.playback.go" label="Go"></hotkey-input>
            <hotkey-input v-model="editingSettings.global.hotkey.playback.load" label="Load"></hotkey-input>
            <hotkey-input
              v-model="editingSettings.global.hotkey.playback.pauseAndResume"
              label="Pause & Resume"
            ></hotkey-input>
            <hotkey-input v-model="editingSettings.global.hotkey.playback.pauseAll" label="Pause All"></hotkey-input>
            <hotkey-input v-model="editingSettings.global.hotkey.playback.resumeAll" label="Resume All"></hotkey-input>
            <hotkey-input v-model="editingSettings.global.hotkey.playback.stop" label="Stop"></hotkey-input>
            <hotkey-input v-model="editingSettings.global.hotkey.playback.stopAll" label="Stop All"></hotkey-input>
            <v-divider></v-divider>
            <h2>Audio Action</h2>
            <hotkey-input
              v-model="editingSettings.global.hotkey.audioAction.toggleRepeat"
              label="ToggleRepeat"
            ></hotkey-input>
          </v-tabs-window-item>
          <v-tabs-window-item
            value="template"
            class="fill-height"
            transition="toggle-slide-y-transition"
            reverse-transition="toggle-slide-y-reverse-transition"
          >
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
          <v-tabs-window-item
            value="audio"
            class="pa-3"
            transition="toggle-slide-y-transition"
            reverse-transition="toggle-slide-y-reverse-transition"
          >
            <v-checkbox v-model="editingSettings.show.audio.monoOutput" label="Downmix stereo to mono"></v-checkbox>
          </v-tabs-window-item>
          <v-tabs-window-item
            value="remote"
            class="pa-3"
            transition="toggle-slide-y-transition"
            reverse-transition="toggle-slide-y-reverse-transition"
          >
            <v-checkbox
              v-model="editingSettings.show.remote.lockCursorToSelection"
              label="Lock Cursor to Selection (on Remote side)"
            ></v-checkbox>
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
import { GlobalSettings } from '../../types/GlobalSettings';

const showModel = useShowModel();
const uiSettings = useUiSettings();

const isSettingsDialogOpen = defineModel<boolean>({ required: true });

const tab = ref('general');
const selectingTemplate = ref<'audio' | 'wait' | null>(null);
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
  uiSettings.update(editingSettings.value.global);
  uiSettings.save();
};
</script>

<style lang="css" module>
.selected-row > td {
  background-color: rgb(var(--v-theme-primary), 0.2);
  color: rgb(var(--v-theme-on-background));
}
</style>
