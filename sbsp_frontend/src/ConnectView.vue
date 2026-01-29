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
        hide-details
        persistent-placeholder
        variant="outlined"
        density="compact"
        autocomplete="off"
        class="flex-grow-0"
        v-model="host"
        :label="t('view.connect.remoteHost')"
        width="400px"
        @keydown.enter="connect(host, port)"
      ></v-text-field>
      <v-text-field
        hide-details
        persistent-placeholder
        variant="outlined"
        density="compact"
        autocomplete="off"
        class="flex-grow-0"
        v-model="port"
        :label="t('view.connect.remotePort')"
        width="100px"
        @keydown.enter="connect(host, port)"
      ></v-text-field>
      <v-btn
        class="ml-auto"
        :text="t('view.connect.connect')"
        color="primary"
        :disabled="host == '' || port == ''"
        @click="connect(host, port)"
      ></v-btn>
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

  let unlisten: (() => void) | null = null;

  const connect = (host: string, port: string | number) => {
    if (host == '' || port == '') return;
    const address = `${host}:${port}`;
    const password = prompt(t('view.connect.passwordPrompt'));
    if (password == null) return;
    if (password == '') {
      api.remote?.connectToServer(address, null);
    } else {
      api.remote?.connectToServer(address, password);
    }
  };

  onMounted(() => {
    if (target == 'websocket') {
      const searchParams = new URLSearchParams(window.location.search);
      const address = searchParams.get('address');
      if (address != null) {
        console.log(`Connecting to ${address}`);
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
    api.remote
      ?.onRemoteDiscoveryUpdate((event) => {
        services.value = event;
      })
      .then((unlisten_func) => {
        unlisten = unlisten_func;
      });
    api.remote?.startServerDiscovery();
  });

  onUnmounted(() => {
    if (unlisten != null) {
      unlisten();
    }
    api.remote?.stopServerDiscovery();
  });
</script>
