<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { onMounted, onUnmounted, ref } from 'vue';
import type { ServiceEntry } from './types/ServiceEntry';
import { useI18n } from 'vue-i18n';
import { useApi } from './api';
import ButtonWrapper from './components/wrapper/ButtonWrapper.vue';
import ProgressSpinnerWrapper from './components/wrapper/ProgressSpinnerWrapper.vue';
import { useToast } from 'primevue/usetoast';
import TextInput from './components/input/TextInput.vue';
import NumberInput from './components/input/NumberInput.vue';

const { t } = useI18n();
const api = useApi();
const toast = useToast();

const host = ref('');
const port = ref(5800);
const services = ref<ServiceEntry[]>([]);

const overlay = ref(false);

const connect = (host: string, port: number) => {
  if (host === '') return;

  const address = `${host}:${port}`;
  let password: string | null;

  if (window.location.hash !== '') {
    password = window.location.hash.substring(1).trim() || null;
  } else if (window.location.href.endsWith('#')) {
    password = null;
  } else {
    const ps_string = prompt(t('view.connect.passwordPrompt'));
    if (ps_string == null) {
      overlay.value = false;
      return;
    }
    password = ps_string.trim() || null;
  }
  if (api.remote != null) {
    overlay.value = true;
    api.remote.connectToServer(address, password).catch((e) => {
      overlay.value = false;
      console.error(e);
      toast.add({
        severity: 'error',
        summary: t('notification.connectionError'),
        detail: e.toString(),
        life: 3000,
      });
    });
  }
};

let unlisten: (() => void) | null;

onMounted(() => {
  api.remote
    ?.onConnectionStatusChanged(() => {
      overlay.value = false;
    })
    .then((ulfn) => (unlisten = ulfn));

  if (__IS_WEBSOCKET__) {
    const searchParams = new URLSearchParams(window.location.search);
    const address = searchParams.get('address');
    if (address != null) {
      overlay.value = true;
      console.log(`Connecting to ${address}`);
      let hostStr = '';
      let portNum = 5800;

      const splitterIndex = address.lastIndexOf(':');

      if (splitterIndex !== -1) {
        hostStr = address.substring(0, splitterIndex);
        const parsedPort = parseInt(address.substring(splitterIndex + 1), 10);
        if (!isNaN(parsedPort)) {
          portNum = parsedPort;
        }
      } else {
        // use whole address as hostname if colon not exists
        hostStr = address;
      }

      host.value = hostStr;
      port.value = portNum;
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

<template>
  <div class="flex h-full w-full flex-col items-stretch">
    <div class="h-full grow overflow-auto">
      <table
        class="w-full border-collapse"
        :class="$style['table']"
      >
        <thead>
          <tr>
            <th class="sticky top-0 z-1">{{ t('view.connect.remoteName') }}</th>
            <th class="sticky top-0 z-1">{{ t('view.connect.remoteHost') }}</th>
            <th class="sticky top-0 z-1 w-25">{{ t('view.connect.remotePort') }}</th>
            <th class="sticky top-0 z-1 w-30"></th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="entry in services"
            :key="entry.host + ':' + entry.port"
            @click="
              host = entry.host;
              port = entry.port;
            "
            @dblclick="
              host = entry.host;
              port = entry.port;
              connect(host, port);
            "
          >
            <td>{{ entry.serverName }}</td>
            <td>{{ entry.host }}</td>
            <td>{{ entry.port }}</td>
            <td>
              <button-wrapper
                variant="outlined"
                size="small"
                :label="t('view.connect.connect')"
                @click="connect(entry.host, entry.port)"
              />
            </td>
          </tr>
        </tbody>
      </table>
    </div>
    <footer class="mr-0 ml-0 flex w-full grow-0 items-center gap-3 p-1">
      <text-input
        class="max-w-160 shrink grow"
        v-model="host"
        :label="t('view.connect.remoteHost')"
      />
      <number-input
        class="w-40 grow-0"
        v-model="port"
        :min="0"
        :max="65535"
        :label="t('view.connect.remotePort')"
      />
      <button-wrapper
        class="ml-auto shrink-0"
        :label="t('view.connect.connect')"
        severity="primary"
        :disabled="host == ''"
        @click="connect(host, port)"
      />
    </footer>
    <Teleport to="body">
      <div
        v-if="overlay"
        class="fixed inset-0 z-1000 h-full w-full flex flex-col align-center justify-center"
        style="background-color: rgb(0, 0, 0, 0.5)"
      >
        <progress-spinner-wrapper
          color="primary.500"
          storke-width="5"
          size="96px"
        />
      </div>
    </Teleport>
  </div>
</template>

<style lang="css" module>
@layer base {
  .table th,
  .table td {
    height: 34px;
    border-bottom: 1px solid rgb(from currentColor r g b / 0.5);
    text-align: left;
    padding-left: calc(var(--spacing) * 2);
    padding-right: calc(var(--spacing) * 2);
  }
  .table tbody tr:nth-of-type(odd) {
    background-color: rgb(from var(--p-surface-500) r g b / 0.2);
  }
}
</style>
