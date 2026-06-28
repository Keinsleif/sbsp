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
const port = ref('');
const services = ref<ServiceEntry[]>([]);

const overlay = ref(false);

const connect = (host: string, port: string | number) => {
  if (host === '' || port === '') return;
  const address = `${host}:${port}`;
  let password: string | null;
  if (window.location.hash !== '') {
    password = window.location.hash.substring(1).trim();
  } else if (window.location.href.endsWith('#')) {
    password = null;
  } else {
    const ps_string = prompt(t('view.connect.passwordPrompt'));
    if (ps_string == null) return;
    if (ps_string !== '') {
      password = ps_string;
    } else {
      password = null;
    }
  }
  overlay.value = true;
  api.remote?.connectToServer(address, password).catch((e) => {
    overlay.value = false;
    console.error(e);
    toast.add({
      severity: 'error',
      summary: t('notification.connectionError'),
      detail: e.toString(),
      life: 3000,
    });
  });
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
              port = entry.port.toString();
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
        class="grow-0"
        v-model="host"
        :label="t('view.connect.remoteHost')"
        @keydown.enter="connect(host, port)"
      />
      <number-input
        class="grow-0"
        v-model="port"
        :min="0"
        :step="1"
        :max-fraction-digits="0"
        :label="t('view.connect.remotePort')"
        @keydown.enter="connect(host, port)"
      />
      <button-wrapper
        class="ml-auto"
        :label="t('view.connect.connect')"
        severity="primary"
        :disabled="host == '' || port == ''"
        @click="connect(host, port)"
      />
    </footer>
    <Teleport to="body">
      <div
        v-if="overlay"
        class="fixed inset-0 z-1000"
        style="background-color: rgb(0, 0, 0, 0.5)"
      >
        <progress-spinner-wrapper
          color="primary"
          size="64"
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
