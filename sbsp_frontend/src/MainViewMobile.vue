<template>
  <v-app
    height="100vh"
    @contextmenu.prevent
  >
    <v-main style="height: 100vh">
      <v-window v-model="activeTab" class="h-100">
        <v-window-item value="controls" class="h-100">
          <controls-panel />
        </v-window-item>
        <v-window-item value="monitor" class="h-100">
          <monitor-panel />
        </v-window-item>
      </v-window>
    </v-main>
    <v-bottom-navigation v-model="activeTab">
      <v-btn value="controls">
        <v-icon :icon="mdiRemote" />

        <span>Controls</span>
      </v-btn>

      <v-btn value="monitor">
        <v-icon :icon="mdiMonitor" />

        <span>Monitor</span>
      </v-btn>
    </v-bottom-navigation>
  </v-app>
</template>

<script setup lang="ts">
import { mdiMonitor, mdiRemote } from '@mdi/js';
import { ref } from 'vue';
import ControlsPanel from './components/mobile/ControlsPanel.vue';
import MonitorPanel from './components/mobile/MonitorPanel.vue';

const activeTab = ref('controls');

</script>
