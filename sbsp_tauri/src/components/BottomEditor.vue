<template>
      <v-sheet class="overflow-hidden">
      <v-tabs v-model="uiState.editorTab" density="compact" class="border">
        <v-tab border density="compact" value="basics">Basics</v-tab>
        <v-tab border density="compact" value="audio">Audio</v-tab>
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
                label="Pre-Wait"
                variant="outlined"
                density="compact"
                class="centered-input"
              ></v-text-field>
              <v-text-field
                hide-details
                persistent-placeholder
                label="Post-Wait"
                variant="outlined"
                density="compact"
                class="centered-input"
              ></v-text-field>
              <v-select
                hide-details
                persistent-placeholder
                label="ContinueMode"
                :items="['Auto-Continue', 'Auto-Follow', 'DoNotContinue']"
                variant="outlined"
                density="compact"
              ></v-select>
            </v-sheet>
            <v-sheet
              flat
              class="d-flex flex-grow-1 flex-column ga-2 justify-start"
            >
              <v-text-field
                hide-details
                persistent-placeholder
                label="Name"
                variant="outlined"
                density="compact"
                class="flex-grow-0"
              ></v-text-field>
              <v-file-input
                hide-details
                :prepend-icon="mdiFile"
                variant="outlined"
                density="compact"
                class="flex-grow-0"
              ></v-file-input>
              <v-textarea
                hide-details
                persistent-placeholder
                no-resize
                label="Notes"
                variant="outlined"
                density="compact"
              ></v-textarea>
            </v-sheet>
          </v-sheet>
        </v-tabs-window-item>
        <v-tabs-window-item
          value="audio"
          reverse-transition="false"
          transition="false"
        >
          <v-sheet height="275px"> </v-sheet>
        </v-tabs-window-item>
        <v-tabs-window-item
          value="levels"
          reverse-transition="false"
          transition="false"
        >
          <v-sheet height="275px"> </v-sheet>
        </v-tabs-window-item>
      </v-tabs-window>
      </v-sheet>
</template>

<script setup lang="ts">
import { mdiFile } from '@mdi/js';
import { useUiState } from '../stores/uistate';

const uiState = useUiState();
</script>