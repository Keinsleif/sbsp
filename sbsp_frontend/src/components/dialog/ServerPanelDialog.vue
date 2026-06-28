<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { onMounted, onUnmounted, ref, useTemplateRef, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { useApi } from '../../api';
import type { ApiServerOptions } from '../../types/ApiServerOptions';
import { mdiEye, mdiEyeOff, mdiPlus } from '@mdi/js';
import CopyTextInput from '../input/CopyTextInput.vue';
import QrViewer from '../display/QrViewer.vue';
import { generateRandomPassword } from '../../utils';
import ServerPanelPasswordRow from './ServerPanelPasswordRow.vue';
import type { PermissionInfo } from '../../types/PermissionInfo';
import { useToast } from 'primevue/usetoast';
import Dialog from 'primevue/dialog';
import { $dt } from '@primeuix/themes';
import TextInput from '../input/TextInput.vue';
import CheckboxWrapper from '../wrapper/CheckboxWrapper.vue';
import PathIcon from '../display/PathIcon.vue';
import ButtonWrapper from '../wrapper/ButtonWrapper.vue';

const { t } = useI18n();
const api = useApi();
const toast = useToast();

const isServerPanelOpen = defineModel<boolean>({ required: true });
const qrRef = useTemplateRef<InstanceType<typeof QrViewer>>('qrRef');

const isPasswordVisible = ref(false);
const isServerInfoDialogOpen = ref(false);
const copied = ref(false);

const isRunning = ref<boolean | null>(null);
const isDiscoverable = ref<boolean | null>(null);
const server_port = ref<string>('');
const server_name = ref<string>('Untitled SBS Player Server');
const server_authMap = ref<PermissionInfo[]>([]);

const server_hostname = ref<string | null>(null);
const server_password = ref<string>('');
const server_url = ref<string | null>(null);

let server_options: ApiServerOptions = {
  port: 5800,
  discoverry: null,
  authMap: [
    {
      password: '',
      permission: 0b0001,
    },
  ],
};

let unlisten: (() => void) | null = null;

const saveServerOptions = async () => {
  if (isDiscoverable.value) {
    server_options.discoverry = server_name.value;
  } else {
    server_options.discoverry = null;
  }
  const parseResult = parseInt(server_port.value);
  if (isNaN(parseResult)) {
    throw Error('Invalid port number');
  }
  server_options.port = parseResult;
  server_options.authMap = server_authMap.value;
  await api.host?.setServerOptions(server_options);
};

const toggleServer = async () => {
  if (isRunning.value) {
    api.host?.stopServer().catch((e) => {
      console.error(e);
      toast.add({
        severity: 'error',
        summary: t('notification.failedToStopServer'),
        detail: e.toString(),
        life: 3000,
      });
    });
  } else {
    isPasswordVisible.value = false;
    saveServerOptions()
      .then(() => {
        api.host?.startServer().catch((e) => {
          console.error(e);
          toast.add({
            severity: 'error',
            summary: t('notification.failedToStartServer'),
            detail: e.toString(),
            life: 3000,
          });
        });
      })
      .catch((e) => {
        toast.add({
          severity: 'error',
          summary: t('notification.failedToStartServer'),
          detail: e.toString(),
          life: 3000,
        });
      });
  }
};

// Generaate 'connectable' url from server_options that contains backend's options
const generateServerUrl = async (password: string) => {
  const hostname = await api.host?.getHostname();
  server_hostname.value = hostname || null;
  if (hostname) {
    const address = `${hostname}:${server_options.port}`;
    server_password.value = password;
    if (password) {
      server_url.value = `http://${address}/?address=${address}#${password}`;
    } else {
      server_url.value = `http://${address}/?address=${address}#`;
    }
  } else {
    server_url.value = '';
  }
};

const copyQr = () => {
  if (qrRef.value == null) return;
  const qrImageData = new XMLSerializer().serializeToString(qrRef.value.$el);
  const qrImageBlob = new Blob([qrImageData], { type: 'image/svg+xml;charset=utf-8' });
  const qrImageUrl = URL.createObjectURL(qrImageBlob);

  const canvas = document.createElement('canvas');
  canvas.width = qrRef.value.$el.clientWidth;
  canvas.height = qrRef.value.$el.clientHeight;
  const context = canvas.getContext('2d');
  if (context == null) return;

  const img = new Image(qrRef.value.$el.clientWidth, qrRef.value.$el.clientHeight);
  img.addEventListener('load', () => {
    console.log('loaded');
    context.drawImage(img, 0, 0);

    canvas.toBlob((blob) => {
      if (blob == null) return;
      navigator.clipboard.write([new ClipboardItem({ 'image/png': blob })]);
    }, 'image/png');

    URL.revokeObjectURL(qrImageUrl);
    copied.value = true;
    setTimeout(() => (copied.value = false), 2000);
  });
  img.src = qrImageUrl;
};

watch(isServerPanelOpen, (value) => {
  if (value) {
    isPasswordVisible.value = false;
  }
});

onMounted(() => {
  api.host
    ?.getServerOptions()
    .then((options) => {
      server_options = options;
      if (options.discoverry != null) {
        isDiscoverable.value = true;
        server_name.value = options.discoverry;
      } else {
        isDiscoverable.value = false;
      }
      server_port.value = options.port.toString();
      server_authMap.value = options.authMap;
    })
    .catch((e) => console.error(e));
  api.host
    ?.isServerRunning()
    .then((state) => (isRunning.value = state))
    .catch((e) => console.error(e));
  api.host
    ?.onServerStatusChanged((status) => {
      if (status === 'started') {
        isRunning.value = true;
      } else if (status === 'stopped') {
        isRunning.value = false;
      }
    })
    .then((unlisten_func) => {
      unlisten = unlisten_func;
    });
});

onUnmounted(() => {
  if (unlisten != null) {
    unlisten();
  }
});
</script>

<template>
  <Dialog
    v-model:visible="isServerPanelOpen"
    class="w-auto"
    :header="t('dialog.server.title')"
    @keydown.stop.esc="isServerPanelOpen = false"
    @contextmenu.prevent
  >
    <div class="flex flex-col gap-4">
      <div class="flex flex-row gap-4">
        <div class="flex flex-col gap-4">
          <div>
            <span>
              {{ t('dialog.server.status') }} :
              <span
                :style="{
                  color:
                    isRunning == null
                      ? ''
                      : isRunning
                        ? $dt('green.500').variable
                        : $dt('red.500').variable,
                }"
              >
                {{
                  isRunning == null
                    ? ''
                    : isRunning
                      ? t('dialog.server.running')
                      : t('dialog.server.stopped')
                }}
              </span>
            </span>
          </div>
          <text-input
            v-model="server_port"
            :disabled="isRunning"
            class="w-25 grow-0"
            :label="t('dialog.server.port')"
          />
          <CheckboxWrapper
            v-model="isDiscoverable"
            :disabled="isRunning"
            :label="t('dialog.server.discoverable')"
          />
          <text-input
            v-model="server_name"
            :disabled="!isDiscoverable || isRunning"
            align-input="left"
            class="mt-1 w-80 grow-0"
            :label="t('dialog.server.serverName')"
          />
        </div>
        <div class="grow overflow-auto border border-(--p-form-field-border-color)">
          <table>
            <thead>
              <tr class="border-b border-(--p-form-field-border-color)">
                <th width="220">
                  {{ t('dialog.server.password') }}
                  <path-icon
                    class="ml-2 cursor-pointer"
                    :icon="isPasswordVisible ? mdiEye : mdiEyeOff"
                    @click="isPasswordVisible = !isPasswordVisible"
                  />
                </th>
                <th
                  width="316"
                  class="border-x border-(--p-form-field-border-color)"
                >
                  {{ t('dialog.server.permission') }}
                </th>
                <th width="108" />
                <th width="45" />
                <th width="45" />
              </tr>
            </thead>
            <tbody>
              <server-panel-password-row
                v-for="(info, i) in server_authMap"
                :key="i"
                v-model:password="info.password"
                v-model:permission="info.permission"
                :is-running="isRunning || false"
                :is-visible="isPasswordVisible"
                @delete="
                  () => {
                    if (server_authMap.length > 0) {
                      server_authMap.splice(i, 1);
                    }
                  }
                "
                @open-info="
                  generateServerUrl(info.password).then(() => {
                    isServerInfoDialogOpen = true;
                  })
                "
              />
              <tr>
                <td
                  colspan="5"
                  class="py-1 text-center"
                >
                  <button-wrapper
                    :icon="mdiPlus"
                    :disabled="isRunning || false"
                    severity="success"
                    variant="outlined"
                    size="small"
                    @click="
                      server_authMap.push({
                        password: generateRandomPassword(),
                        permission: 0b0001,
                      })
                    "
                  />
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
      <div class="mt-auto mr-0 ml-0 flex w-full grow-0 items-center gap-3">
        <button-wrapper
          class="ml-auto"
          :label="isRunning ? t('dialog.server.stop') : t('dialog.server.start')"
          :severity="isRunning ? 'danger' : 'success'"
          @click="toggleServer"
        />
      </div>
    </div>
    <Dialog
      v-model:visible="isServerInfoDialogOpen"
      :header="t('dialog.server.connectInfo')"
      class="w-auto"
    >
      <div class="flex w-120 flex-col items-stretch gap-3 p-3">
        <div class="mt-2 flex flex-row items-center">
          <copy-text-input
            v-model="server_hostname"
            readonly
            :placeholder="t('dialog.server.info.hostnameUnavailable')"
            class="grow"
            :label="t('dialog.server.info.host')"
          />
          <span class="px-2">:</span>
          <copy-text-input
            v-model="server_port"
            readonly
            class="w-25 grow-0"
            :label="t('dialog.server.port')"
          />
        </div>
        <copy-text-input
          v-model="server_password"
          readonly
          :placeholder="t('dialog.server.info.passwordNotSet')"
          class="mt-2 grow"
          :label="t('dialog.server.password')"
        />
        <copy-text-input
          v-model="server_url"
          readonly
          :placeholder="t('dialog.server.info.urlUnavailable')"
          class="mt-2 grow"
          :label="t('dialog.server.info.url')"
        />
        <div class="text-center">
          <qr-viewer
            ref="qrRef"
            v-model="server_url"
            width="240px"
            height="240px"
          />
        </div>
        <button-wrapper
          :text="t('dialog.server.info.copyQr')"
          @click="copyQr"
        >
        </button-wrapper>
      </div>
    </Dialog>
  </Dialog>
</template>
