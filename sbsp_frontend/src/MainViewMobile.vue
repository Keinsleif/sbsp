<template>
  <v-app
    height="100vh"
    @contextmenu.prevent
  >
    <v-app-bar
      app
      border
      flat
      height="100"
    >
      <div
        class="d-flex align-center border flex-grow-0"
        style="height: 100px; width: 100%;"
      >
        <div
          class="d-flex align-end pl-3 pr-3 text-center justify-center flex-grow-1"
          style="font-size: 4em; line-height: 1"
        >
          <span>{{ String(time.getHours()).padStart(2, '0') }}</span>:<span>{{ String(time.getMinutes()).padStart(2, '0') }}</span>.<span style="font-size: 32pt; line-height: 1">{{ String(time.getSeconds()).padStart(2, '0') }}</span>
        </div>
      </div>
    </v-app-bar>
    <v-main>
      <v-window v-model="activeTab" class="h-100" :touch="false">
        <v-window-item value="list" class="d-flex h-100">
          <cue-list />
        </v-window-item>
        <v-window-item value="controls" class="h-100">
          <controls-panel />
        </v-window-item>
        <v-window-item value="monitor" class="h-100">
          <monitor-panel />
        </v-window-item>
      </v-window>
    </v-main>
    <v-bottom-navigation v-model="activeTab">
      <v-btn @click="activeTab = 'list'">
        <v-icon :icon="mdiFormatListBulleted" />
        <span>List</span>
      </v-btn>
      <v-btn @click="activeTab = 'controls'">
        <v-icon :icon="mdiRemote" />
        <span>Controls</span>
      </v-btn>
      <v-btn @click="activeTab = 'monitor'">
        <v-icon :icon="mdiMonitor" />
        <span>Monitor</span>
      </v-btn>
    </v-bottom-navigation>
  </v-app>
</template>

<script setup lang="ts">
import { useNow } from '@vueuse/core';
import { mdiFormatListBulleted, mdiMonitor, mdiRemote } from '@mdi/js';
import { ref } from 'vue';
import ControlsPanel from './components/mobile/ControlsPanel.vue';
import MonitorPanel from './components/mobile/MonitorPanel.vue';
import CueList from './components/mobile/CueList.vue';

const activeTab = ref('list');
const time = useNow();
</script>
