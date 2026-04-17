<template>
  <div class="d-flex flex-column w-100 h-100">
    <v-table
      fixed-header
      density="compact"
      class="flex-grow-1"
      height="100%"
      striped="even"
    >
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
          :key="entry.host + ':' + entry.port"
          v-ripple
          @click="
            host = entry.host;
            port = entry.port.toString();
          "
          @dblclick="connect(entry.host, entry.port)"
        >
          <th>{{ entry.serverName }}</th>
          <th>{{ entry.host }}</th>
          <th>{{ entry.port }}</th>
        </tr>
      </tbody>
    </v-table>
    <v-footer class="flex-grow-0 d-flex align-center ml-0 mr-0 w-100 ga-3">
      <v-text-field
        v-model="host"
        hide-details
        persistent-placeholder
        variant="outlined"
        density="compact"
        autocomplete="off"
        class="flex-grow-0"
        :label="t('view.connect.remoteHost')"
        width="400px"
        @keydown.enter="connect(host, port)"
      />
      <v-text-field
        v-model="port"
        hide-details
        persistent-placeholder
        variant="outlined"
        density="compact"
        autocomplete="off"
        class="flex-grow-0"
        :label="t('view.connect.remotePort')"
        width="100px"
        @keydown.enter="connect(host, port)"
      />
      <v-btn
        class="ml-auto"
        :text="t('view.connect.connect')"
        color="primary"
        :disabled="host == '' || port == ''"
        @click="connect(host, port)"
      />
    </v-footer>
  </div>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue';
import { ServiceEntry } from './types/ServiceEntry';
import { useI18n } from 'vue-i18n';
import { target, useApi } from './api';

const { t } = useI18n();
const api = useApi();

const host = ref('');
const port = ref('');
const services = ref<ServiceEntry[]>([]);

const connect = (host: string, port: string | number) => {
  if (host == '' || port == '') return;
  const address = `${host}:${port}`;
  if (window.location.hash != '') {
    api.remote?.connectToServer(address, window.location.hash.substring(1).trim());
  } else if (window.location.href.endsWith('#')) {
    api.remote?.connectToServer(address, null);
  } else {
    let password = prompt(t('view.connect.passwordPrompt'));
    if (password == null) return;
    if (password != '') {
      api.remote?.connectToServer(address, password);
    } else {
      api.remote?.connectToServer(address, null);
    }
  }
};

onMounted(() => {
  if (target == 'websocket') {
    const searchParams = new URLSearchParams(window.location.search);
    const address = searchParams.get('address');
    if (address != null) {
      console.log(`Connecting to ${address}`);
      host.value = address.split(':')[0] || '';
      port.value = address.split(':')[1] || '5800';
      if (window.location.hash != '') {
        api.remote?.connectToServer(address, window.location.hash.substring(1).trim());
      } else if (window.location.href.endsWith('#')) {
        api.remote?.connectToServer(address, null);
      } else {
        let password = prompt(t('view.connect.passwordPrompt'));
        if (password == null) return;
        if (password != '') {
          api.remote?.connectToServer(address, password);
        } else {
          api.remote?.connectToServer(address, null);
        }
      }
    }
  }

  api.remote?.startServerDiscovery((event) => {
    services.value = event;
  });
});

onUnmounted(() => {
  api.remote?.stopServerDiscovery();
});
</script>
