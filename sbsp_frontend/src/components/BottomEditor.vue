<template>
  <v-sheet class="overflow-hidden">
    <div v-show="selectedCue != null && selectedCue.params.type == 'audio'">
      <v-tabs v-model="audioEditorTab" density="compact">
        <v-tab density="compact" value="basics">{{ t('main.bottomEditor.basics.title') }}</v-tab>
        <v-tab density="compact" value="audio">
          {{ t('main.bottomEditor.audio.title') }}
        </v-tab>
        <v-tab density="compact" value="time">
          {{ t('main.bottomEditor.timeLevels.title') }}
        </v-tab>
      </v-tabs>
    </div>
    <div v-show="selectedCue != null && selectedCue.params.type == 'fade'">
      <v-tabs v-model="fadeEditorTab" density="compact">
        <v-tab density="compact" value="basics">{{ t('main.bottomEditor.basics.title') }}</v-tab>
        <v-tab density="compact" value="fade">
          {{ t('main.bottomEditor.fade.title') }}
        </v-tab>
      </v-tabs>
    </div>
    <div
      v-show="
        selectedCue != null &&
        (selectedCue.params.type == 'start' ||
          selectedCue.params.type == 'stop' ||
          selectedCue.params.type == 'pause' ||
          selectedCue.params.type == 'load')
      "
    >
      <v-tabs v-model="playbackEditorTab" density="compact">
        <v-tab density="compact" value="basics">{{ t('main.bottomEditor.basics.title') }}</v-tab>
        <v-tab density="compact" value="playback">
          {{ t('main.bottomEditor.playback.title') }}
        </v-tab>
      </v-tabs>
    </div>
    <div v-show="selectedCue != null && selectedCue.params.type == 'group'">
      <v-tabs v-model="groupEditorTab" density="compact">
        <v-tab density="compact" value="basics">{{ t('main.bottomEditor.basics.title') }}</v-tab>
        <v-tab density="compact" value="group">
          {{ t('main.bottomEditor.group.title') }}
        </v-tab>
      </v-tabs>
    </div>
    <div v-show="selectedCue != null && selectedCue.params.type == 'wait'">
      <v-tabs v-model="otherEditorTab" density="compact">
        <v-tab density="compact" value="basics">{{ t('main.bottomEditor.basics.title') }}</v-tab>
      </v-tabs>
    </div>
    <div v-show="selectedCue == null"></div>
    <v-tabs-window class="border-t-sm" v-show="selectedCue != null" v-model="editorTab">
      <v-tabs-window-item value="basics" reverse-transition="false" transition="false">
        <basic-editor v-model="selectedCue" :sequence-override="props.sequenceOverride" @update="edited" />
      </v-tabs-window-item>
      <v-tabs-window-item value="audio" reverse-transition="false" transition="false">
        <audio-basic-editor v-model="selectedCue" @update="edited" />
      </v-tabs-window-item>
      <v-tabs-window-item value="time" reverse-transition="false" transition="false">
        <audio-time-level-editor v-model="selectedCue" @update="edited" />
      </v-tabs-window-item>
      <v-tabs-window-item value="fade" reverse-transition="false" transition="false">
        <fade-basic-editor v-model="selectedCue" @update="edited" />
      </v-tabs-window-item>
      <v-tabs-window-item value="playback" reverse-transition="false" transition="false">
        <playback-basic-editor v-model="selectedCue" @update="edited" />
      </v-tabs-window-item>
      <v-tabs-window-item value="group" reverse-transition="false" transition="false">
        <group-basic-editor v-model="selectedCue" @update="edited"></group-basic-editor>
      </v-tabs-window-item>
    </v-tabs-window>
  </v-sheet>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue';
import BasicEditor from './editor/BasicEditor.vue';
import AudioTimeLevelEditor from './editor/AudioTimeLevelEditor.vue';
import AudioBasicEditor from './editor/AudioBasicEditor.vue';
import type { Cue } from '../types/Cue';
import FadeBasicEditor from './editor/FadeBasicEditor.vue';
import { useI18n } from 'vue-i18n';
import PlaybackBasicEditor from './editor/PlaybackBasicEditor.vue';
import GroupBasicEditor from './editor/GroupBasicEditor.vue';
import type { CueSequence } from '../types/CueSequence';

const { t } = useI18n();
const selectedCue = defineModel<Cue | null>();
const props = withDefaults(
  defineProps<{
    sequenceOverride?: CueSequence | null;
  }>(),
  {
    sequenceOverride: null,
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

const editorTab = computed(() => {
  switch (selectedCue.value?.params.type) {
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
    case undefined:
      return undefined;
    default:
      return otherEditorTab.value;
  }
});
</script>

<style lang="css" module>
.centered-input input {
  text-align: center;
}
</style>
