<template>
  <v-sheet class="overflow-hidden">
    <v-tabs v-model="editorTab" density="compact" class="border">
      <v-tab border density="compact" value="basics">Basics</v-tab>
      <v-tab border density="compact" value="audio" v-if="selectedCue != null && selectedCue.params.type == 'audio'"
        >Audio</v-tab
      >
      <v-tab border density="compact" value="time" v-if="selectedCue != null && selectedCue.params.type == 'audio'"
        >Time & Levels</v-tab
      >
    </v-tabs>
    <v-tabs-window v-if="selectedCue != null" v-model="editorTab">
      <v-tabs-window-item value="basics" reverse-transition="false" transition="false">
        <BasicEditor />
      </v-tabs-window-item>
      <v-tabs-window-item
        v-if="selectedCue != null && selectedCue.params.type == 'audio'"
        value="audio"
        reverse-transition="false"
        transition="false"
      >
        <AudioBasicEditor />
      </v-tabs-window-item>
      <v-tabs-window-item
        v-if="selectedCue != null && selectedCue.params.type == 'audio'"
        value="time"
        reverse-transition="false"
        transition="false"
      >
        <AudioTimeLevelEditor />
      </v-tabs-window-item>
    </v-tabs-window>
  </v-sheet>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue';
import { useUiState } from '../stores/uistate';
import { useShowModel } from '../stores/showmodel';
import BasicEditor from './editor/BasicEditor.vue';
import AudioTimeLevelEditor from './editor/AudioTimeLevelEditor.vue';
import AudioBasicEditor from './editor/AudioBasicEditor.vue';

const showModel = useShowModel();
const uiState = useUiState();

const editorTab = ref('basics');

const selectedCue = computed(() => {
  return uiState.selected != null ? showModel.cues.find((cue) => cue.id === uiState.selected) : null;
});
</script>

<style lang="css" module>
.centered-input input {
  text-align: center;
}
</style>
