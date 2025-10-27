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
              v-model="editingSettings.general.advanceCursorWhenGo"
              label="Advance cursor when GO"
              hide-details
            ></v-checkbox>
            <v-divider></v-divider>
            <v-checkbox
              v-model="editingSettings.general.lockCursorToSelection"
              label="Lock Cursor to Selection (on Main side)"
              hide-details
            ></v-checkbox>
            <v-divider></v-divider>
            <v-checkbox
              v-model="editingSettings.general.copyAssetsWhenAdd"
              label="Copy assets when adding Cue"
              hide-details
            ></v-checkbox>
            <text-input
              v-model="editingSettings.general.copyAssetsDestination"
              class="ma-2"
              align-input="left"
              width="500px"
              label="Assets directory"
              hint="Relative path to the directory for copying assets."
              persistent-hint
              show-details
            ></text-input>
            <v-divider></v-divider>
            <v-number-input
              hide-details
              inset
              persistent-placeholder
              class="ma-2"
              v-model="editingSettings.general.seekAmount"
              :min="0"
              width="160px"
              label="Seek amount"
              density="compact"
              variant="outlined"
              autocomplete="off"
              @keydown.stop
            ></v-number-input>
          </v-tabs-window-item>
          <v-tabs-window-item
            value="hotkey"
            class="pa-3"
            transition="toggle-slide-y-transition"
            reverse-transition="toggle-slide-y-reverse-transition"
          >
            <h2>Playback</h2>
            <hotkey-input v-model="editingSettings.hotkey.playback.go" label="Go"></hotkey-input>
            <hotkey-input v-model="editingSettings.hotkey.playback.load" label="Load"></hotkey-input>
            <hotkey-input
              v-model="editingSettings.hotkey.playback.pauseAndResume"
              label="Pause & Resume"
            ></hotkey-input>
            <hotkey-input v-model="editingSettings.hotkey.playback.pauseAll" label="Pause All"></hotkey-input>
            <hotkey-input v-model="editingSettings.hotkey.playback.resumeAll" label="Resume All"></hotkey-input>
            <hotkey-input v-model="editingSettings.hotkey.playback.stop" label="Stop"></hotkey-input>
            <hotkey-input v-model="editingSettings.hotkey.playback.stopAll" label="Stop All"></hotkey-input>
            <v-divider></v-divider>
            <h2>Audio Action</h2>
            <hotkey-input v-model="editingSettings.hotkey.audioAction.toggleRepeat" label="ToggleRepeat"></hotkey-input>
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
                      {{ editingSettings.template.audio.number }}
                    </td>
                    <td headers="cuelist_name" width="auto">
                      {{
                        editingSettings.template.audio.name != null
                          ? editingSettings.template.audio.name
                          : '** built from cue param **'
                      }}
                    </td>
                    <td headers="cuelist_pre_wait" class="text-center pa-1" width="100px">
                      <div>
                        {{ secondsToFormat(editingSettings.template.audio.preWait) }}
                      </div>
                    </td>
                    <td headers="cuelist_duration" class="text-center pa-1" width="100px">
                      <div>
                        {{ secondsToFormat(calculateDuration(editingSettings.template.audio.params, null)) }}
                      </div>
                    </td>
                    <td headers="cuelist_post_wait" class="text-center pa-1" width="100px">
                      <div>
                        {{
                          editingSettings.template.audio.sequence.type == 'doNotContinue'
                            ? '--:--.--'
                            : editingSettings.template.audio.sequence.type == 'autoContinue'
                              ? secondsToFormat(editingSettings.template.audio.sequence.postWait)
                              : secondsToFormat(calculateDuration(editingSettings.template.audio.params, null))
                        }}
                      </div>
                    </td>
                    <td headers="cuelist_repeat">
                      <v-icon
                        v-if="
                          editingSettings.template.audio.params.type == 'audio' &&
                          editingSettings.template.audio.params.repeat
                        "
                        :icon="mdiRepeat"
                      />
                    </td>
                    <td headers="cuelist_sequence">
                      <v-icon
                        v-if="editingSettings.template.audio.sequence.type == 'autoFollow'"
                        :icon="mdiArrowExpandDown"
                      />
                      <v-icon
                        v-if="editingSettings.template.audio.sequence.type == 'autoContinue'"
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
                      {{ editingSettings.template.wait.number }}
                    </td>
                    <td headers="cuelist_name" width="auto">
                      {{
                        editingSettings.template.wait.name != null
                          ? editingSettings.template.wait.name
                          : '** built from cue param **'
                      }}
                    </td>
                    <td headers="cuelist_pre_wait" class="text-center pa-1" width="100px">
                      <div>
                        {{ secondsToFormat(editingSettings.template.wait.preWait) }}
                      </div>
                    </td>
                    <td headers="cuelist_duration" class="text-center pa-1" width="100px">
                      <div>
                        {{
                          secondsToFormat(
                            editingSettings.template.wait.params.type == 'wait'
                              ? editingSettings.template.wait.params.duration
                              : null,
                          )
                        }}
                      </div>
                    </td>
                    <td headers="cuelist_post_wait" class="text-center pa-1" width="100px">
                      <div>
                        {{
                          editingSettings.template.wait.sequence.type == 'doNotContinue'
                            ? '--:--.--'
                            : editingSettings.template.wait.sequence.type == 'autoContinue'
                              ? secondsToFormat(editingSettings.template.wait.sequence.postWait)
                              : secondsToFormat(calculateDuration(editingSettings.template.wait.params, null))
                        }}
                      </div>
                    </td>
                    <td headers="cuelist_repeat">
                      <v-icon
                        v-if="
                          editingSettings.template.wait.params.type == 'audio' &&
                          editingSettings.template.wait.params.repeat
                        "
                        :icon="mdiRepeat"
                      />
                    </td>
                    <td headers="cuelist_sequence">
                      <v-icon
                        v-if="editingSettings.template.wait.sequence.type == 'autoFollow'"
                        :icon="mdiArrowExpandDown"
                      />
                      <v-icon
                        v-if="editingSettings.template.wait.sequence.type == 'autoContinue'"
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
            <v-checkbox v-model="editingSettings.audio.monoOutput" label="Downmix stereo to mono"></v-checkbox>
          </v-tabs-window-item>
          <v-tabs-window-item
            value="remote"
            class="pa-3"
            transition="toggle-slide-y-transition"
            reverse-transition="toggle-slide-y-reverse-transition"
          >
            <v-checkbox
              v-model="editingSettings.remote.lockCursorToSelection"
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

const showModel = useShowModel();

const isSettingsDialogOpen = defineModel<boolean>({ required: true });

const tab = ref('general');
const selectingTemplate = ref<'audio' | 'wait' | null>(null);
const editingSettings = ref<ShowSettings>(structuredClone(toRaw(showModel.settings)));

const getSelectingCue = () => {
  if (selectingTemplate.value == 'audio') {
    return editingSettings.value.template.audio;
  } else if (selectingTemplate.value == 'wait') {
    return editingSettings.value.template.wait;
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
    editingSettings.value = structuredClone(toRaw(newSettings));
  },
);

watch(isSettingsDialogOpen, (newState) => {
  if (newState) {
    editingSettings.value = structuredClone(toRaw(showModel.settings));
  }
});

const saveSettings = () => {
  invoke('update_settings', { newSettings: editingSettings.value }).catch((e) => console.error(e));
};
</script>

<style lang="css" module>
.selected-row > td {
  background-color: rgb(var(--v-theme-primary), 0.2);
  color: rgb(var(--v-theme-on-background));
}
</style>
