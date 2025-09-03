<template>
  <v-sheet class="overflow-hidden">
    <v-tabs v-model="editorTab" density="compact">
      <v-tab density="compact" value="basics">Basics</v-tab>
      <v-tab density="compact" value="audio" v-if="selectedCue != null && selectedCue.params.type == 'audio'"
        >Audio</v-tab
      >
      <v-tab density="compact" value="time" v-if="selectedCue != null && selectedCue.params.type == 'audio'"
        >Time & Levels</v-tab
      >
    </v-tabs>
    <v-tabs-window class="border-t-sm" v-if="selectedCue != null" v-model="editorTab">
      <v-tabs-window-item value="basics" reverse-transition="false" transition="false">
        <basic-editor :selected-id="props.selectedId" />
      </v-tabs-window-item>
      <v-tabs-window-item
        v-if="selectedCue != null && selectedCue.params.type == 'audio'"
        value="audio"
        reverse-transition="false"
        transition="false"
      >
        <audio-basic-editor :selected-id="props.selectedId" />
      </v-tabs-window-item>
      <v-tabs-window-item
        v-if="selectedCue != null && selectedCue.params.type == 'audio'"
        value="time"
        reverse-transition="false"
        transition="false"
      >
        <audio-time-level-editor :selected-id="props.selectedId" />
      </v-tabs-window-item>
    </v-tabs-window>
  </v-sheet>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue';
import { useShowModel } from '../stores/showmodel';
import BasicEditor from './editor/BasicEditor.vue';
import AudioTimeLevelEditor from './editor/AudioTimeLevelEditor.vue';
import AudioBasicEditor from './editor/AudioBasicEditor.vue';

const props = withDefaults(
  defineProps<{
    selectedId: string | null;
  }>(),
  {
    selectedId: null,
  },
);

const showModel = useShowModel();

const editorTab = ref('basics');

const selectedCue = computed(() => {
  return props.selectedId != null ? showModel.cues.find((cue) => cue.id === props.selectedId) : null;
});
</script>

<style lang="css" module>
.centered-input input {
  text-align: center;
}
</style>
