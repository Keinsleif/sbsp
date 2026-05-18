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
    <v-overlay
      :model-value="overlay"
      persistent
      class="align-center justify-center"
    >
      <v-progress-circular
        color="primary"
        size="64"
        indeterminate
      />
    </v-overlay>
  </div>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue';
import { ServiceEntry } from './types/ServiceEntry';
import { useI18n } from 'vue-i18n';
import { target, useApi } from './api';
import { useUiState } from './stores/uistate';

const { t } = useI18n();
const api = useApi();
const uiState = useUiState();

const host = ref('');
const port = ref('');
const services = ref<ServiceEntry[]>([]);

const overlay = ref(false);

const errorHandler = (e: unknown) => {
  overlay.value = false;
  console.error(e);
  uiState.error(`${e}`);
};

const connect = (host: string, port: string | number) => {
  if (host == '' || port == '') return;
  const address = `${host}:${port}`;
  let password: string | null;
  if (window.location.hash != '') {
    password = window.location.hash.substring(1).trim();
  } else if (window.location.href.endsWith('#')) {
    password = null;
  } else {
    let ps_string = prompt(t('view.connect.passwordPrompt'));
    if (ps_string == null) return;
    if (ps_string != '') {
      password = ps_string;
    } else {
      password = null;
    }
  }
  overlay.value = true;
  api.remote?.connectToServer(address, password).catch(errorHandler);
};

let unlisten: (() => void) | null;

onMounted(() => {
  api.remote
    ?.onConnectionStatusChanged(() => {
      overlay.value = false;
    })
    .then(ulfn => (unlisten = ulfn));

  if (target == 'websocket') {
    const searchParams = new URLSearchParams(window.location.search);
    const address = searchParams.get('address');
    if (address != null) {
      overlay.value = true;
      console.log(`Connecting to ${address}`);
      host.value = address.split(':')[0] || '';
      port.value = address.split(':')[1] || '5800';
      connect(host.value, port.value);
    }
  }

  api.remote?.startServerDiscovery((event) => {
    services.value = event;
  });
});

onUnmounted(() => {
  if (unlisten != null) {
    unlisten();
  }
  api.remote?.stopServerDiscovery();
});
</script>
