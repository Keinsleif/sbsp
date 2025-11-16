<template>
  <div class="d-flex flex-column w-100 h-100">
    <div class="d-flex flex-column pa-4 ga-4">
      <div>
        <span>
          {{ t('view.server.status') }} :
          <span :class="isRunning == null ? '' : isRunning ? 'text-green' : 'text-red'">
            {{ isRunning == null ? '' : isRunning ? t('view.server.running') : t('view.server.stopped') }}
          </span>
        </span>
      </div>
      <text-input
        v-model="server_port"
        :disabled="isRunning"
        class="flex-grow-0"
        :label="t('view.server.port')"
        width="100px"
        @update="invoke('set_server_port', { port: parseInt(server_port) }).catch((e) => console.error(e))"
      ></text-input>
      <v-checkbox
        v-model="is_discoverable"
        :disabled="isRunning"
        :label="t('view.server.discoverable')"
        density="compact"
        hide-details
        @update="invoke('set_discovery_option', { discoveryOption: server_name }).catch((e) => console.error(e))"
      ></v-checkbox>
      <text-input
        v-model="server_name"
        :disabled="!is_discoverable || isRunning"
        align-input="left"
        class="flex-grow-0"
        :label="t('view.server.serverName')"
        width="480px"
        @update="invoke('set_discovery_option', { discoveryOption: server_name }).catch((e) => console.error(e))"
      ></text-input>
      <v-snackbar-queue v-model="error_messages" timeout="2000" color="error"></v-snackbar-queue>
    </div>
    <v-footer class="flex-grow-0 d-flex align-center ml-0 mr-0 w-100 mt-auto ga-3">
      <v-btn
        class="ml-auto"
        :text="isRunning ? t('view.server.stop') : t('view.server.start')"
        :color="isRunning ? 'red' : 'green'"
        @click="toggleServer"
      ></v-btn>
      <v-btn :text="t('general.close')" variant="outlined" @click="close"></v-btn>
    </v-footer>
  </div>
</template>

<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { onMounted, onUnmounted, ref, watch } from 'vue';
import TextInput from './components/input/TextInput.vue';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();

const isRunning = ref<boolean | null>(null);
const server_port = ref<string>('');
const is_discoverable = ref<boolean | null>(null);
const server_name = ref<string>('');
const error_messages = ref<string[]>([]);

watch(
  () => server_name.value,
  () => {
    console.log(server_name.value);
  },
);

let unlisten: UnlistenFn | null = null;

const close = () => {
  const webviewWindow = getCurrentWebviewWindow();
  webviewWindow.close();
};

const toggleServer = () => {
  if (isRunning.value) {
    invoke('stop_server').catch((e) => {
      console.error(e);
      error_messages.value.push(e);
    });
  } else {
    invoke('start_server').catch((e) => {
      console.error(e);
      error_messages.value.push(e);
    });
  }
};

onMounted(() => {
  invoke<number>('get_server_port')
    .then((port) => (server_port.value = port.toString()))
    .catch((e) => console.error(e));
  invoke<string>('get_discovery_option')
    .then((name) => {
      if (name != null) {
        is_discoverable.value = true;
        server_name.value = name;
      } else {
        is_discoverable.value = false;
      }
    })
    .catch((e) => console.error(e));
  invoke<boolean>('is_server_running')
    .then((state) => (isRunning.value = state))
    .catch((e) => console.error(e));
  listen<'started' | 'stopped'>('backend-server-status-changed', (event) => {
    if (event.payload == 'started') {
      isRunning.value = true;
    } else if (event.payload == 'stopped') {
      isRunning.value = false;
    }
  }).then((unlisten_func) => {
    unlisten = unlisten_func;
  });
});

onUnmounted(() => {
  if (unlisten != null) {
    unlisten();
  }
});
</script>
