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
            :text="t('dialog.settings.tab.audio')"
            value="audio"
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
            value="audio"
            class="pa-4"
          >
            <v-number-input
              v-model="editingSettings.show.audio.outputChannels"
              class="mt-4"
              :label="t('dialog.settings.show.audio.outputChannels')"
              density="compact"
              variant="outlined"
              hide-details
              width="200px"
              autocomplete="off"
              :min="1"
              :max="8"
              :precision="0"
            />
            <v-checkbox
              v-model="editingSettings.show.audio.monoOutput"
              :label="t('dialog.settings.show.audio.monoOutput')"
              hide-details
            />
            <v-number-input
              v-model="editingSettings.show.audio.lufsTarget"
              class="mt-4"
              :label="t('dialog.settings.show.audio.targetLufs')"
              suffix="LUFS"
              density="compact"
              variant="outlined"
              hide-details
              width="200px"
              autocomplete="off"
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
            <v-sheet
              class="d-flex flex-column w-100"
              height="100%"
            >
              <v-table
                fixed-header
                density="compact"
                class="flex-grow-1 border"
                height="100%"
              >
                <thead>
                  <tr>
                    <th id="cuelist_type">
                      {{ t('dialog.settings.global.template.type') }}
                    </th>
                    <th
                      id="cuelist_number"
                      width="60px"
                    >
                      {{ t('main.number') }}
                    </th>
                    <th id="cuelist_name">
                      {{ t('main.name') }}
                    </th>
                    <th
                      id="cuelist_pre_wait"
                      class="text-center"
                    >
                      {{ t('main.preWait') }}
                    </th>
                    <th
                      id="cuelist_duration"
                      class="text-center"
                    >
                      {{ t('main.duration') }}
                    </th>
                    <th
                      id="cuelist_repeat"
                      width="53px"
                    >
                      <v-icon :icon="mdiRepeat" />
                    </th>
                    <th
                      id="cuelist_chain"
                      width="53px"
                    >
                      <v-icon :icon="mdiChevronDoubleDown" />
                    </th>
                  </tr>
                </thead>
                <tbody>
                  <tr
                    :class="[selectingTemplate == 'audio' ? $style['selected-row'] : '']"
                    @mousedown="selectingTemplate = 'audio'"
                  >
                    <td
                      headers="cuelist_type"
                      width="160px"
                    >
                      {{ t('dialog.settings.global.template.audio') }}
                    </td>
                    <td
                      headers="cuelist_number"
                      class="text-center"
                      width="50px"
                    >
                      {{ editingSettings.global.template.audio.number }}
                    </td>
                    <td
                      headers="cuelist_name"
                      width="auto"
                    >
                      {{
                        editingSettings.global.template.audio.name != null
                          ? editingSettings.global.template.audio.name
                          : t('dialog.settings.global.template.builtFromCueParam')
                      }}
                    </td>
                    <td
                      headers="cuelist_pre_wait"
                      class="text-center pa-1"
                      width="100px"
                    >
                      <div>
                        {{ secondsToFormat(editingSettings.global.template.audio.preWait) }}
                      </div>
                    </td>
                    <td
                      headers="cuelist_duration"
                      class="text-center pa-1"
                      width="100px"
                    >
                      <div>
                        {{ secondsToFormat(calculateDuration(editingSettings.global.template.audio.params, null)) }}
                      </div>
                    </td>
                    <td headers="cuelist_repeat">
                      <v-icon
                        v-show="
                          editingSettings.global.template.audio.params.type == 'audio' &&
                            editingSettings.global.template.audio.params.repeat
                        "
                        :icon="mdiRepeat"
                      />
                    </td>
                    <td headers="cuelist_chain">
                      <v-icon
                        v-show="editingSettings.global.template.audio.chain.type == 'afterComplete'"
                        :icon="mdiArrowExpandDown"
                      />
                      <v-icon
                        v-show="editingSettings.global.template.audio.chain.type == 'afterStart'"
                        :icon="mdiArrowDown"
                      />
                    </td>
                  </tr>
                  <tr
                    :class="[selectingTemplate == 'wait' ? $style['selected-row'] : '']"
                    @mousedown="selectingTemplate = 'wait'"
                  >
                    <td
                      headers="cuelist_type"
                      width="160px"
                    >
                      {{ t('dialog.settings.global.template.wait') }}
                    </td>
                    <td
                      headers="cuelist_number"
                      class="text-center"
                      width="50px"
                    >
                      {{ editingSettings.global.template.wait.number }}
                    </td>
                    <td
                      headers="cuelist_name"
                      width="auto"
                    >
                      {{
                        editingSettings.global.template.wait.name != null
                          ? editingSettings.global.template.wait.name
                          : t('dialog.settings.global.template.builtFromCueParam')
                      }}
                    </td>
                    <td
                      headers="cuelist_pre_wait"
                      class="text-center pa-1"
                      width="100px"
                    >
                      <div>
                        {{ secondsToFormat(editingSettings.global.template.wait.preWait) }}
                      </div>
                    </td>
                    <td
                      headers="cuelist_duration"
                      class="text-center pa-1"
                      width="100px"
                    >
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
                    <td headers="cuelist_repeat" />
                    <td headers="cuelist_chain">
                      <v-icon
                        v-show="editingSettings.global.template.wait.chain.type == 'afterComplete'"
                        :icon="mdiArrowExpandDown"
                      />
                      <v-icon
                        v-show="editingSettings.global.template.wait.chain.type == 'afterStart'"
                        :icon="mdiArrowDown"
                      />
                    </td>
                  </tr>
                  <tr
                    :class="[selectingTemplate == 'fade' ? $style['selected-row'] : '']"
                    @mousedown="selectingTemplate = 'fade'"
                  >
                    <td
                      headers="cuelist_type"
                      width="160px"
                    >
                      {{ t('dialog.settings.global.template.fade') }}
                    </td>
                    <td
                      headers="cuelist_number"
                      class="text-center"
                      width="50px"
                    >
                      {{ editingSettings.global.template.fade.number }}
                    </td>
                    <td
                      headers="cuelist_name"
                      width="auto"
                    >
                      {{
                        editingSettings.global.template.fade.name != null
                          ? editingSettings.global.template.fade.name
                          : t('dialog.settings.global.template.builtFromCueParam')
                      }}
                    </td>
                    <td
                      headers="cuelist_pre_wait"
                      class="text-center pa-1"
                      width="100px"
                    >
                      <div>
                        {{ secondsToFormat(editingSettings.global.template.fade.preWait) }}
                      </div>
                    </td>
                    <td
                      headers="cuelist_duration"
                      class="text-center pa-1"
                      width="100px"
                    >
                      <div>
                        {{
                          secondsToFormat(
                            editingSettings.global.template.fade.params.type == 'fade'
                              ? editingSettings.global.template.fade.params.fadeParam.duration
                              : null,
                          )
                        }}
                      </div>
                    </td>
                    <td headers="cuelist_repeat" />
                    <td headers="cuelist_chain">
                      <v-icon
                        v-show="editingSettings.global.template.fade.chain.type == 'afterComplete'"
                        :icon="mdiArrowExpandDown"
                      />
                      <v-icon
                        v-show="editingSettings.global.template.fade.chain.type == 'afterStart'"
                        :icon="mdiArrowDown"
                      />
                    </td>
                  </tr>
                  <tr
                    :class="[selectingTemplate == 'start' ? $style['selected-row'] : '']"
                    @mousedown="selectingTemplate = 'start'"
                  >
                    <td
                      headers="cuelist_type"
                      width="160px"
                    >
                      {{ t('dialog.settings.global.template.start') }}
                    </td>
                    <td
                      headers="cuelist_number"
                      class="text-center"
                      width="50px"
                    >
                      {{ editingSettings.global.template.start.number }}
                    </td>
                    <td
                      headers="cuelist_name"
                      width="auto"
                    >
                      {{
                        editingSettings.global.template.fade.name != null
                          ? editingSettings.global.template.fade.name
                          : t('dialog.settings.global.template.builtFromCueParam')
                      }}
                    </td>
                    <td
                      headers="cuelist_pre_wait"
                      class="text-center pa-1"
                      width="100px"
                    >
                      <div>
                        {{ secondsToFormat(editingSettings.global.template.start.preWait) }}
                      </div>
                    </td>
                    <td
                      headers="cuelist_duration"
                      class="text-center pa-1"
                      width="100px"
                    >
                      <div>
                        {{ secondsToFormat(null) }}
                      </div>
                    </td>
                    <td headers="cuelist_repeat" />
                    <td headers="cuelist_chain">
                      <v-icon
                        v-show="editingSettings.global.template.start.chain.type == 'afterComplete'"
                        :icon="mdiArrowExpandDown"
                      />
                      <v-icon
                        v-show="editingSettings.global.template.start.chain.type == 'afterStart'"
                        :icon="mdiArrowDown"
                      />
                    </td>
                  </tr>
                  <tr
                    :class="[selectingTemplate == 'stop' ? $style['selected-row'] : '']"
                    @mousedown="selectingTemplate = 'stop'"
                  >
                    <td
                      headers="cuelist_type"
                      width="160px"
                    >
                      {{ t('dialog.settings.global.template.stop') }}
                    </td>
                    <td
                      headers="cuelist_number"
                      class="text-center"
                      width="50px"
                    >
                      {{ editingSettings.global.template.stop.number }}
                    </td>
                    <td
                      headers="cuelist_name"
                      width="auto"
                    >
                      {{
                        editingSettings.global.template.stop.name != null
                          ? editingSettings.global.template.stop.name
                          : t('dialog.settings.global.template.builtFromCueParam')
                      }}
                    </td>
                    <td
                      headers="cuelist_pre_wait"
                      class="text-center pa-1"
                      width="100px"
                    >
                      <div>
                        {{ secondsToFormat(editingSettings.global.template.stop.preWait) }}
                      </div>
                    </td>
                    <td
                      headers="cuelist_duration"
                      class="text-center pa-1"
                      width="100px"
                    >
                      <div>
                        {{ secondsToFormat(null) }}
                      </div>
                    </td>
                    <td headers="cuelist_repeat" />
                    <td headers="cuelist_chain">
                      <v-icon
                        v-show="editingSettings.global.template.stop.chain.type == 'afterComplete'"
                        :icon="mdiArrowExpandDown"
                      />
                      <v-icon
                        v-show="editingSettings.global.template.stop.chain.type == 'afterStart'"
                        :icon="mdiArrowDown"
                      />
                    </td>
                  </tr>
                  <tr
                    :class="[selectingTemplate == 'pause' ? $style['selected-row'] : '']"
                    @mousedown="selectingTemplate = 'pause'"
                  >
                    <td
                      headers="cuelist_type"
                      width="160px"
                    >
                      {{ t('dialog.settings.global.template.pause') }}
                    </td>
                    <td
                      headers="cuelist_number"
                      class="text-center"
                      width="50px"
                    >
                      {{ editingSettings.global.template.pause.number }}
                    </td>
                    <td
                      headers="cuelist_name"
                      width="auto"
                    >
                      {{
                        editingSettings.global.template.pause.name != null
                          ? editingSettings.global.template.pause.name
                          : t('dialog.settings.global.template.builtFromCueParam')
                      }}
                    </td>
                    <td
                      headers="cuelist_pre_wait"
                      class="text-center pa-1"
                      width="100px"
                    >
                      <div>
                        {{ secondsToFormat(editingSettings.global.template.pause.preWait) }}
                      </div>
                    </td>
                    <td
                      headers="cuelist_duration"
                      class="text-center pa-1"
                      width="100px"
                    >
                      <div>
                        {{ secondsToFormat(null) }}
                      </div>
                    </td>
                    <td headers="cuelist_repeat" />
                    <td headers="cuelist_chain">
                      <v-icon
                        v-show="editingSettings.global.template.pause.chain.type == 'afterComplete'"
                        :icon="mdiArrowExpandDown"
                      />
                      <v-icon
                        v-show="editingSettings.global.template.pause.chain.type == 'afterStart'"
                        :icon="mdiArrowDown"
                      />
                    </td>
                  </tr>
                  <tr
                    :class="[selectingTemplate == 'load' ? $style['selected-row'] : '']"
                    @mousedown="selectingTemplate = 'load'"
                  >
                    <td
                      headers="cuelist_type"
                      width="160px"
                    >
                      {{ t('dialog.settings.global.template.load') }}
                    </td>
                    <td
                      headers="cuelist_number"
                      class="text-center"
                      width="50px"
                    >
                      {{ editingSettings.global.template.load.number }}
                    </td>
                    <td
                      headers="cuelist_name"
                      width="auto"
                    >
                      {{
                        editingSettings.global.template.load.name != null
                          ? editingSettings.global.template.load.name
                          : t('dialog.settings.global.template.builtFromCueParam')
                      }}
                    </td>
                    <td
                      headers="cuelist_pre_wait"
                      class="text-center pa-1"
                      width="100px"
                    >
                      <div>
                        {{ secondsToFormat(editingSettings.global.template.load.preWait) }}
                      </div>
                    </td>
                    <td
                      headers="cuelist_duration"
                      class="text-center pa-1"
                      width="100px"
                    >
                      <div>
                        {{ secondsToFormat(null) }}
                      </div>
                    </td>
                    <td headers="cuelist_repeat" />
                    <td headers="cuelist_chain">
                      <v-icon
                        v-show="editingSettings.global.template.load.chain.type == 'afterComplete'"
                        :icon="mdiArrowExpandDown"
                      />
                      <v-icon
                        v-show="editingSettings.global.template.load.chain.type == 'afterStart'"
                        :icon="mdiArrowDown"
                      />
                    </td>
                  </tr>
                  <tr
                    :class="[selectingTemplate == 'group' ? $style['selected-row'] : '']"
                    @mousedown="selectingTemplate = 'group'"
                  >
                    <td
                      headers="cuelist_type"
                      width="160px"
                    >
                      {{ t('dialog.settings.global.template.group') }}
                    </td>
                    <td
                      headers="cuelist_number"
                      class="text-center"
                      width="50px"
                    >
                      {{ editingSettings.global.template.group.number }}
                    </td>
                    <td
                      headers="cuelist_name"
                      width="auto"
                    >
                      {{
                        editingSettings.global.template.group.name != null
                          ? editingSettings.global.template.group.name
                          : t('dialog.settings.global.template.builtFromCueParam')
                      }}
                    </td>
                    <td
                      headers="cuelist_pre_wait"
                      class="text-center pa-1"
                      width="100px"
                    >
                      <div>
                        {{ secondsToFormat(editingSettings.global.template.group.preWait) }}
                      </div>
                    </td>
                    <td
                      headers="cuelist_duration"
                      class="text-center pa-1"
                      width="100px"
                    >
                      <div>
                        {{ secondsToFormat(null) }}
                      </div>
                    </td>
                    <td headers="cuelist_repeat" />
                    <td headers="cuelist_chain">
                      <v-icon
                        v-show="editingSettings.global.template.group.chain.type == 'afterComplete'"
                        :icon="mdiArrowExpandDown"
                      />
                      <v-icon
                        v-show="editingSettings.global.template.group.chain.type == 'afterStart'"
                        :icon="mdiArrowDown"
                      />
                    </td>
                  </tr>
                </tbody>
              </v-table>
              <div
                style="height: 250px"
                class="flex-grow-0 mb-0 border"
              >
                <bottom-editor v-model="selectingCue" />
              </div>
            </v-sheet>
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
            saveSettings();
            isSettingsDialogOpen = false;
          "
        />
      </v-footer>
    </v-sheet>
  </v-dialog>
</template>

<script setup lang="ts">
import { ref, toRaw, watch } from 'vue';
import { useShowModel } from '../../stores/showmodel';
import type { ShowSettings } from '../../types/ShowSettings';
import HotkeyInput from '../input/HotkeyInput.vue';
import { mdiChevronDoubleDown, mdiRepeat, mdiArrowExpandDown, mdiArrowDown } from '@mdi/js';
import { secondsToFormat, calculateDuration } from '../../utils';
import BottomEditor from '../pc/BottomEditor.vue';
import type { Cue } from '../../types/Cue';
import TextInput from '../input/TextInput.vue';
import { useUiSettings } from '../../stores/uiSettings';
import type { GlobalSettings } from '../../types/GlobalSettings';
import { useI18n } from 'vue-i18n';
import { useApi } from '../../api';

const { t } = useI18n();
const api = useApi();
const showModel = useShowModel();
const uiSettings = useUiSettings();

const isSettingsDialogOpen = defineModel<boolean>({ required: true });

const tab = ref('showGeneral');
const selectingTemplate = ref<'audio' | 'wait' | 'fade' | 'start' | 'stop' | 'pause' | 'load' | 'group' | null>(null);
const showModelName = ref<string>(showModel.name);
const editingSettings = ref<{
  show: ShowSettings;
  global: GlobalSettings;
}>({ show: structuredClone(toRaw(showModel.settings)), global: uiSettings.clone() });

const getSelectingCue = () => {
  if (selectingTemplate.value == 'audio') {
    return editingSettings.value.global.template.audio;
  } else if (selectingTemplate.value == 'wait') {
    return editingSettings.value.global.template.wait;
  } else if (selectingTemplate.value == 'fade') {
    return editingSettings.value.global.template.fade;
  } else if (selectingTemplate.value == 'start') {
    return editingSettings.value.global.template.start;
  } else if (selectingTemplate.value == 'stop') {
    return editingSettings.value.global.template.stop;
  } else if (selectingTemplate.value == 'pause') {
    return editingSettings.value.global.template.pause;
  } else if (selectingTemplate.value == 'load') {
    return editingSettings.value.global.template.load;
  } else if (selectingTemplate.value == 'group') {
    return editingSettings.value.global.template.group;
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

const saveSettings = () => {
  api.updateShowSettings(editingSettings.value.show);
  api.updateModelName(showModelName.value);
  uiSettings.update(editingSettings.value.global);
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
