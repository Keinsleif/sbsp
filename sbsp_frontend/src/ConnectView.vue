<template>
  <div class="d-flex flex-column w-100 h-100">
    <v-table fixed-header density="compact" class="flex-grow-1" height="100%" striped="even">
      <thead>
        <tr>
          <th>{{ t('view.connect.remoteName') }}</th>
          <th>{{ t('view.connect.remoteHost') }}</th>
          <th>{{ t('view.connect.remotePort') }}</th>
        </tr>
      </thead>
      <tbody>
        <tr
          v-for="entry in services"
          v-ripple
          :key="entry.host + ':' + entry.port"
          @click="
            host = entry.host;
            port = entry.port.toString();
          "
          @dblclick="connect(entry.host + ':' + entry.port)"
        >
          <th>{{ entry.serverName }}</th>
          <th>{{ entry.host }}</th>
          <th>{{ entry.port }}</th>
        </tr>
      </tbody>
    </v-table>
    <v-footer class="flex-grow-0 d-flex align-center ml-0 mr-0 w-100 ga-3">
      <text-input
        class="flex-grow-0"
        v-model="host"
        :label="t('view.connect.remoteHost')"
        width="400px"
        align-input="left"
      ></text-input>
      <text-input class="flex-grow-0" v-model="port" :label="t('view.connect.remotePort')" width="100px"></text-input>
      <v-btn
        class="ml-auto"
        :text="t('view.connect.connect')"
        color="primary"
        :disabled="host == '' || port == ''"
        @click="connect(host + ':' + port)"
      ></v-btn>
    </v-footer>
  </div>
</template>

<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { onMounted, onUnmounted, ref } from 'vue';
import { ServiceEntry } from './types/ServiceEntry';
import TextInput from './components/input/TextInput.vue';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();

const host = ref('');
const port = ref('');
const services = ref<ServiceEntry[]>([]);

let unlisten: UnlistenFn | null = null;

const connect = (address: string) => {
  invoke('connect_to_server', { address: address }).catch((e) => console.error(e));
};

onMounted(() => {
  listen<ServiceEntry[]>('remote-discovery', (event) => {
    services.value = event.payload;
  }).then((unlisten_func) => {
    unlisten = unlisten_func;
  });
  invoke('start_server_discovery').catch((e) => console.error(e));
});

onUnmounted(() => {
  if (unlisten != null) {
    unlisten();
  }
  invoke('stop_server_discovery').catch((e) => console.error(e));
});
</script>
