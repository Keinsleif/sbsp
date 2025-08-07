<template>
  <v-app height="100vh">
    <v-app-bar app border flat height="200">
      <ToolHeader />
    </v-app-bar>

    <v-main>
      <v-sheet height="100%" class="d-flex">
        <CueList />
      </v-sheet>
    </v-main>

    <v-footer app border class="py-1">
      <v-sheet class="d-flex align-center ml-0 mr-0 w-100">
        <v-sheet class="ml-0 mr-auto d-flex align-center">
          <v-switch
            inset
            hide-details
            color="primary"
            :true-icon="mdiEye"
            :false-icon="mdiFileEdit"
          ></v-switch>
          <v-spacer></v-spacer>
          <v-btn :icon="mdiDockTop" size="small" variant="text"></v-btn>
        </v-sheet>
        <v-sheet class="ml-auto mr-auto"> 50 cues </v-sheet>
        <v-sheet class="mr-0 ml-auto d-flex align-center">
          <v-btn :icon="mdiDockBottom" size="small" variant="text" @click="toggleEditor"></v-btn>
          <v-btn
            :icon="mdiDockRight"
            size="small"
            variant="text"
            @click="toggleRightSidebar"
          ></v-btn>
          <v-btn :icon="mdiCog" size="small" variant="text"></v-btn>
        </v-sheet>
      </v-sheet>
    </v-footer>

    <v-navigation-drawer
      v-model="rightSidebarOpen"
      app
      permanent
      location="right"
      width="300"
    >
      <v-tabs grow fixed-tabs v-model="sidebarTab" density="compact">
        <v-tab border density="compact" value="activeCues">Active Cues</v-tab>
        <v-tab border density="compact" value="levels">Levels</v-tab>
      </v-tabs>
      <v-tabs-window v-model="sidebarTab">
        <v-tabs-window-item value="activeCues" class="overflow-y-auto" transition="false" reverse-transition="false">
          <v-card v-for="i in 3" :key="i" :value="i" class="border">
            <v-card-title class="text-subtitle-2">
              {{ i }}・Intro Music (Playing)
            </v-card-title>
            <v-progress-linear
              color="primary"
              :model-value="i * 20"
              height="8"
            ></v-progress-linear>
          </v-card>
        </v-tabs-window-item>
      </v-tabs-window>
    </v-navigation-drawer>
    <v-navigation-drawer
      v-model="editorOpen"
      app
      permanent
      location="bottom"
      width="301"
    >
      <v-sheet class="overflow-hidden">
      <v-tabs v-model="editTab" density="compact" class="border">
        <v-tab border density="compact" value="basics">Basics</v-tab>
        <v-tab border density="compact" value="audio">Audio</v-tab>
        <v-tab border density="compact" value="levels">Levels</v-tab>
      </v-tabs>
      <v-tabs-window v-model="editTab">
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
    </v-navigation-drawer>
  </v-app>
</template>

<script setup lang="ts">
import { ref } from "vue";
import {
  mdiCog,
  mdiEye,
  mdiFileEdit,
  mdiDockTop,
  mdiDockRight,
  mdiDockBottom,
  mdiFile,
} from "@mdi/js";
import ToolHeader from "./components/ToolHeader.vue";
import CueList from "./components/CueList.vue";

const rightSidebarOpen = ref(true);
const editorOpen = ref(true);
const editTab = ref("basics");
const sidebarTab = ref("activeCues");

const toggleRightSidebar = () => {
  rightSidebarOpen.value = !rightSidebarOpen.value;
};
const toggleEditor = () => {
  editorOpen.value = !editorOpen.value;
}
</script>

<style>
.centered-input input {
  text-align: center;
}
</style>
