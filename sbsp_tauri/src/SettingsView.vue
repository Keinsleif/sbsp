<template>
  <v-sheet>
    <v-tabs v-model="tab" direction="vertical">
      <v-tab text="General" value="general"></v-tab>
      <v-tab text="Hotkeys" value="hotkeys"></v-tab>
    </v-tabs>
    <v-tabs-window v-model="tab">
      <v-tabs-window-item value="general">
      </v-tabs-window-item>
      <v-tabs-window-item value="hotkey">
      </v-tabs-window-item>
    </v-tabs-window>
  </v-sheet>
</template>

<script setup lang="ts">
import { ref } from 'vue';

const tab = ref('general');
</script>
