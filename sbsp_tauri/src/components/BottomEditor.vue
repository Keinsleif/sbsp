<template>
      <v-sheet class="overflow-hidden">
      <v-tabs v-model="uiState.editorTab" density="compact" class="border">
        <v-tab border density="compact" value="basics">Basics</v-tab>
        <v-tab border density="compact" value="audio" v-if="selectedCue != null && selectedCue.params.type=='audio'">Audio</v-tab>
        <v-tab border density="compact" value="levels">Levels</v-tab>
      </v-tabs>
      <v-tabs-window v-model="uiState.editorTab">
        <v-tabs-window-item
          value="basics"
          reverse-transition="false"
          transition="false"
        >
          <v-sheet flat class="d-flex flex-row pa-4 ga-4">
            <v-sheet flat class="d-flex flex-column ga-2" width="175px">
              <v-text-field
                hide-details
                persistent-placeholder
                :model-value="selectedCue != null ? selectedCue.number : ''"
                label="Number"
                variant="outlined"
                density="compact"
                class="centered-input"
              ></v-text-field>
              <v-text-field
                hide-details
                persistent-placeholder
                readonly
                label="Duration"
                variant="outlined"
                density="compact"
                class="centered-input"
              ></v-text-field>
              <v-text-field
                hide-details
                persistent-placeholder
                :model-value="selectedCue != null ? selectedCue.preWait : ''"
                label="Pre-Wait"
                variant="outlined"
                density="compact"
                class="centered-input"
              ></v-text-field>
              <v-select
                hide-details
                persistent-placeholder
                :model-value="selectedCue != null ? selectedCue.sequence.type : ''"
                label="ContinueMode"
                :items="[{value: 'doNotContinue', name: 'DoNotContinue'},{value: 'autoContinue', name: 'Auto-Continue'}, {value: 'autoFollow', name: 'Auto-Follow'}]"
                item-value="value"
                item-title="name"
                variant="outlined"
                density="compact"
              ></v-select>
              <v-text-field
                hide-details
                persistent-placeholder
                :model-value="selectedCue != null && selectedCue.sequence.type == 'autoFollow' ? selectedCue.sequence.postWait : ''"
                label="Post-Wait"
                variant="outlined"
                density="compact"
                class="centered-input"
              ></v-text-field>
            </v-sheet>
            <v-sheet
              flat
              class="d-flex flex-grow-1 flex-column ga-2 justify-start"
            >
              <v-text-field
                hide-details
                persistent-placeholder
                :model-value="selectedCue != null ? selectedCue.name : ''"
                label="Name"
                variant="outlined"
                density="compact"
                class="flex-grow-0"
              ></v-text-field>
              <v-textarea
                hide-details
                persistent-placeholder
                no-resize
                :model-value="selectedCue != null ? selectedCue.notes : ''"
                label="Notes"
                variant="outlined"
                density="compact"
              ></v-textarea>
            </v-sheet>
          </v-sheet>
        </v-tabs-window-item>
        <v-tabs-window-item
          v-if="selectedCue != null && selectedCue.params.type=='audio'"
          value="audio"
          reverse-transition="false"
          transition="false"
        >
          <v-sheet flat class="d-flex flex-column pa-4">
            <v-sheet flat class="d-flex flex-column">
              <v-text-field
                hide-details
                persistent-placeholder
                :model-value="selectedCue.params.target"
                label="Target"
                variant="outlined"
                density="compact"
                class="centered-input"
              ></v-text-field>
            </v-sheet>
          </v-sheet>
        </v-tabs-window-item>
        <v-tabs-window-item
          value="levels"
          reverse-transition="false"
          transition="false"
        >
          <v-sheet> </v-sheet>
        </v-tabs-window-item>
      </v-tabs-window>
      </v-sheet>
</template>

<script setup lang="ts">
import { useUiState } from '../stores/uistate';
import { useShowModel } from '../stores/showmodel';
import { computed } from 'vue';

const showModel = useShowModel();
const uiState = useUiState();

const selectedCue = computed(() => {
  return uiState.selected != null ? showModel.cues[uiState.selected] : null;
})
</script>