<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import type { Cue } from '../../types/Cue';
import type { CueChain } from '../../types/CueChain';
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import Tabs from 'primevue/tabs';
import TabList from 'primevue/tablist';
import Tab from 'primevue/tab';
import TabPanels from 'primevue/tabpanels';
import BasicEditor from '../editor/BasicEditor.vue';
import TabPanel from 'primevue/tabpanel';
import AudioBasicEditor from '../editor/AudioBasicEditor.vue';
import AudioTimeLevelEditor from '../editor/AudioTimeLevelEditor.vue';
import FadeBasicEditor from '../editor/FadeBasicEditor.vue';
import PlaybakcBasicEditor from '../editor/PlaybakcBasicEditor.vue';
import GroupBasicEditor from '../editor/GroupBasicEditor.vue';

const { t } = useI18n();
const selectedCue = defineModel<Cue | null>();
const props = withDefaults(
  defineProps<{
    chainOverride?: CueChain | null;
  }>(),
  {
    chainOverride: null,
  },
);
const emit = defineEmits(['update']);

const edited = () => {
  document.body.focus();
  emit('update');
};

const audioEditorTab = ref('basics');
const fadeEditorTab = ref('basics');
const playbackEditorTab = ref('basics');
const groupEditorTab = ref('basics');
const otherEditorTab = ref('basics');

const editorTab = computed({
  get() {
    if (selectedCue.value == null) {
      return 'blank';
    }
    switch (selectedCue.value.params.type) {
      case 'audio':
        return audioEditorTab.value;
      case 'fade':
        return fadeEditorTab.value;
      case 'start':
      case 'load':
      case 'pause':
      case 'stop':
        return playbackEditorTab.value;
      case 'group':
        return groupEditorTab.value;
      default:
        return otherEditorTab.value;
    }
  },
  set(value) {
    if (selectedCue.value == null) return;
    switch (selectedCue.value.params.type) {
      case 'audio':
        audioEditorTab.value = value;
        break;
      case 'fade':
        fadeEditorTab.value = value;
        break;
      case 'start':
      case 'load':
      case 'pause':
      case 'stop':
        playbackEditorTab.value = value;
        break;
      case 'group':
        groupEditorTab.value = value;
        break;
      default:
        otherEditorTab.value = value;
    }
  },
});
</script>

<template>
  <div class="h-full overflow-auto border-x border-(--p-form-field-border-color)">
    <tabs
      v-model:value="editorTab"
      class="flex h-full flex-col overflow-hidden"
    >
      <tab-list class="shrink-0 grow-0">
        <tab
          value="basics"
          v-show="selectedCue != null"
        >{{ t('main.bottomEditor.basics.title') }}</tab>
        <tab
          value="audio"
          v-show="selectedCue != null && selectedCue.params.type == 'audio'"
          >{{ t('main.bottomEditor.audio.title') }}</tab
        >
        <tab
          value="time"
          v-show="selectedCue != null && selectedCue.params.type == 'audio'"
          >{{ t('main.bottomEditor.timeLevels.title') }}</tab
        >
        <tab
          value="fade"
          v-show="selectedCue != null && selectedCue.params.type == 'fade'"
          >{{ t('main.bottomEditor.fade.title') }}</tab
        >
        <tab
          value="playback"
          v-show="
            selectedCue != null &&
            (selectedCue.params.type == 'start' ||
              selectedCue.params.type == 'stop' ||
              selectedCue.params.type == 'pause' ||
              selectedCue.params.type == 'load')
          "
          >{{ t('main.bottomEditor.playback.title') }}</tab
        >
        <tab
          value="group"
          v-show="selectedCue != null && selectedCue.params.type == 'group'"
          >{{ t('main.bottomEditor.group.title') }}</tab
        >
      </tab-list>
      <tab-panels class="grow overflow-auto p-0">
        <tab-panel value="blank">
          <div style="margin: auto; width: fit-content">No Selection</div>
        </tab-panel>
        <tab-panel value="basics">
          <BasicEditor
            v-model="selectedCue"
            :chain-override="props.chainOverride"
            @update="edited"
          />
        </tab-panel>
        <tab-panel value="audio">
          <AudioBasicEditor
            v-model="selectedCue"
            @update="edited"
          />
        </tab-panel>
        <tab-panel value="time">
          <AudioTimeLevelEditor
            v-model="selectedCue"
            @update="edited"
          />
        </tab-panel>
        <tab-panel value="fade">
          <FadeBasicEditor
            v-model="selectedCue"
            @update="edited"
          />
        </tab-panel>
        <tab-panel value="playback">
          <PlaybakcBasicEditor
            v-model="selectedCue"
            @update="edited"
          />
        </tab-panel>
        <tab-panel value="group">
          <GroupBasicEditor
            v-model="selectedCue"
            @update="edited"
          />
        </tab-panel>
      </tab-panels>
    </tabs>
  </div>
</template>
