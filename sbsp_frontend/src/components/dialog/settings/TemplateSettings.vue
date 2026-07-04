<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { mdiChevronDoubleDown, mdiRepeat } from '@mdi/js';
import BottomEditor from '../../pc/BottomEditor.vue';
import type { Cue } from '../../../types/Cue';
import { ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type { ShowSettings } from '../../../types/ShowSettings';
import type { GlobalHostSettings } from '../../../types/GlobalHostSettings';
import type { GlobalRemoteSettings } from '../../../types/GlobalRemoteSettings';
import PathIcon from '@/components/display/PathIcon.vue';
import TemplateRow from './TemplateRow.vue';

const { t } = useI18n();
const editingSettings = defineModel<{
  show: ShowSettings;
  global: GlobalHostSettings | GlobalRemoteSettings;
}>({ required: true });
const selectingTemplate = ref<
  'audio' | 'wait' | 'fade' | 'start' | 'stop' | 'pause' | 'load' | 'group' | null
>(null);

const getSelectingCue = () => {
  if (selectingTemplate.value === 'audio') {
    return editingSettings.value.global.template.audio;
  } else if (selectingTemplate.value === 'wait') {
    return editingSettings.value.global.template.wait;
  } else if (selectingTemplate.value === 'fade') {
    return editingSettings.value.global.template.fade;
  } else if (selectingTemplate.value === 'start') {
    return editingSettings.value.global.template.start;
  } else if (selectingTemplate.value === 'stop') {
    return editingSettings.value.global.template.stop;
  } else if (selectingTemplate.value === 'pause') {
    return editingSettings.value.global.template.pause;
  } else if (selectingTemplate.value === 'load') {
    return editingSettings.value.global.template.load;
  } else if (selectingTemplate.value === 'group') {
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

<template>
  <div class="flex h-full w-full flex-col overflow-hidden">
    <div class="w-full grow overflow-auto border border-(--p-form-field-border-color)">
      <table
        class="w-full"
        :class="$style['cuelist']"
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
              <path-icon :icon="mdiRepeat" />
            </th>
            <th
              id="cuelist_chain"
              width="53px"
            >
              <path-icon :icon="mdiChevronDoubleDown" />
            </th>
          </tr>
        </thead>
        <tbody>
          <template-row
            v-model="editingSettings.global.template.audio"
            :selected="selectingTemplate === 'audio'"
            @pointerdown="selectingTemplate = 'audio'"
          />
          <template-row
            v-model="editingSettings.global.template.wait"
            :selected="selectingTemplate === 'wait'"
            @pointerdown="selectingTemplate = 'wait'"
          />
          <template-row
            v-model="editingSettings.global.template.fade"
            :selected="selectingTemplate === 'fade'"
            @pointerdown="selectingTemplate = 'fade'"
          />
          <template-row
            v-model="editingSettings.global.template.start"
            :selected="selectingTemplate === 'start'"
            @pointerdown="selectingTemplate = 'start'"
          />
          <template-row
            v-model="editingSettings.global.template.stop"
            :selected="selectingTemplate === 'stop'"
            @pointerdown="selectingTemplate = 'stop'"
          />
          <template-row
            v-model="editingSettings.global.template.pause"
            :selected="selectingTemplate === 'pause'"
            @pointerdown="selectingTemplate = 'pause'"
          />
          <template-row
            v-model="editingSettings.global.template.load"
            :selected="selectingTemplate === 'load'"
            @pointerdown="selectingTemplate = 'load'"
          />
          <template-row
            v-model="editingSettings.global.template.group"
            :selected="selectingTemplate === 'group'"
            @pointerdown="selectingTemplate = 'group'"
          />
        </tbody>
      </table>
    </div>
    <div class="mb-0 h-62 grow-0 overflow-hidden border border-(--p-form-field-border-color)">
      <bottom-editor v-model="selectingCue" />
    </div>
  </div>
</template>

<style lang="css" module>
.selected-row {
  background-color: rgb(from var(--p-primary-color) r g b / 0.3);
}

.cuelist {
  scroll-padding-top: 32px;
  font-size: 0.8em;
  min-width: 800px;
}

.cuelist th {
  position: sticky;
  background-color: color-mix(in oklab, var(--p-content-background) 60%, var(--p-surface-500));
  top: 0;
  z-index: 10;
}

@layer base {
  .cuelist th,
  .cuelist td {
    height: 32px;
    text-align: left;
    border-bottom: 1px solid var(--p-form-field-border-color);
    padding-left: calc(var(--spacing) * 2);
    padding-right: calc(var(--spacing) * 2);
  }
}
</style>
