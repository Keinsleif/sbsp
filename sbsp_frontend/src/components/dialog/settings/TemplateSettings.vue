<template>
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
          @pointerdown="selectingTemplate = 'audio'"
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
          @pointerdown="selectingTemplate = 'wait'"
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
          @pointerdown="selectingTemplate = 'fade'"
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
          @pointerdown="selectingTemplate = 'start'"
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
          @pointerdown="selectingTemplate = 'stop'"
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
          @pointerdown="selectingTemplate = 'pause'"
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
          @pointerdown="selectingTemplate = 'load'"
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
          @pointerdown="selectingTemplate = 'group'"
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
</template>

<script setup lang="ts">
import { mdiChevronDoubleDown, mdiRepeat, mdiArrowExpandDown, mdiArrowDown } from '@mdi/js';
import { secondsToFormat, calculateDuration } from '../../../utils';
import BottomEditor from '../../pc/BottomEditor.vue';
import type { Cue } from '../../../types/Cue';
import { ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type { ShowSettings } from '../../../types/ShowSettings';
import type { GlobalHostSettings } from '../../../types/GlobalHostSettings';
import type { GlobalRemoteSettings } from '../../../types/GlobalRemoteSettings';

const { t } = useI18n();
const editingSettings = defineModel<{
  show: ShowSettings;
  global: GlobalHostSettings | GlobalRemoteSettings;
}>({ required: true });
const selectingTemplate = ref<'audio' | 'wait' | 'fade' | 'start' | 'stop' | 'pause' | 'load' | 'group' | null>(null);

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
</script>

<style lang="css" module>
  .selected-row > td {
    background-color: rgb(var(--v-theme-primary), 0.2);
    color: rgb(var(--v-theme-on-background));
  }
</style>
