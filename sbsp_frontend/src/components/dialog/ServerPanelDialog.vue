<template>
  <v-dialog
    v-model="isServerPanelOpen"
    width="auto"
    @keydown.esc.stop="isServerPanelOpen = false"
    @keydown.stop
    @contextmenu.prevent
  >
    <v-sheet class="d-flex flex-column">
      <div class="d-flex flex-column pa-4 ga-4">
        <h2>{{ t('dialog.server.title') }}</h2>
        <div>
          <span>
            {{ t('dialog.server.status') }} :
            <span :class="isRunning == null ? '' : isRunning ? 'text-green' : 'text-red'">
              {{ isRunning == null ? '' : isRunning ? t('dialog.server.running') : t('dialog.server.stopped') }}
            </span>
          </span>
        </div>
        <text-input
          v-model="server_port"
          :disabled="isRunning"
          class="flex-grow-0"
          :label="t('dialog.server.port')"
          width="100px"
        ></text-input>
        <v-checkbox
          v-model="isDiscoverable"
          :disabled="isRunning"
          :label="t('dialog.server.discoverable')"
          density="compact"
          hide-details
        ></v-checkbox>
        <text-input
          v-model="server_name"
          :disabled="!isDiscoverable || isRunning"
          align-input="left"
          class="flex-grow-0 mt-1"
          :label="t('dialog.server.serverName')"
          width="480px"
        ></text-input>
        <div class="d-flex flex-row align-center ga-3 mt-1">
          <text-input
            v-model="server_password"
            :disabled="isRunning"
            :type="isPasswordVisible ? 'text' : 'password'"
            :placeholder="t('dialog.server.info.passwordNotSet')"
            :append-inner-icon="isPasswordVisible ? mdiEye : mdiEyeOff"
            align-input="left"
            class="flex-grow-1"
            :label="t('dialog.server.password')"
            @click:append-inner="isPasswordVisible = !isPasswordVisible"
          ></text-input>
          <v-btn
            :text="t('dialog.server.generate')"
            color="primary"
            @click="server_password = generateRandomPassword()"
          ></v-btn>
        </div>
        <v-btn
          :text="t('dialog.server.showInfo')"
          :disabled="!isRunning"
          variant="tonal"
          @click="
            generateServerUrl().then(() => {
              isServerInfoDialogOpen = true;
            })
          "
        ></v-btn>
      </div>
      <v-footer class="flex-grow-0 d-flex align-center ml-0 mr-0 w-100 mt-auto ga-3">
        <v-btn
          class="ml-auto"
          :text="isRunning ? t('dialog.server.stop') : t('dialog.server.start')"
          :color="isRunning ? 'red' : 'green'"
          @click="toggleServer"
        ></v-btn>
        <v-btn :text="t('general.close')" variant="outlined" @click="isServerPanelOpen = false"></v-btn>
      </v-footer>
    </v-sheet>
    <v-dialog v-model="isServerInfoDialogOpen" width="auto">
      <v-sheet class="d-flex flex-column pa-3 ga-3 align-stretch" width="480px">
        <h3>Server Information</h3>
        <div class="d-flex flex-row align-center mt-2">
          <copy-text-input
            v-model="server_hostname"
            readonly
            :placeholder="t('dialog.server.info.hostnameUnavailable')"
            align-input="left"
            class="flex-grow-1"
            :label="t('dialog.server.info.host')"
          ></copy-text-input>
          <span class="px-2">:</span>
          <copy-text-input
            v-model="server_port"
            readonly
            align-input="left"
            class="flex-grow-0"
            :label="t('dialog.server.port')"
            width="100px"
          ></copy-text-input>
        </div>
        <copy-text-input
          v-model="server_password"
          readonly
          :placeholder="t('dialog.server.info.passwordNotSet')"
          align-input="left"
          class="flex-grow-1 mt-2"
          :label="t('dialog.server.password')"
        ></copy-text-input>
        <copy-text-input
          v-model="server_url"
          readonly
          :placeholder="t('dialog.server.info.urlUnavailable')"
          align-input="left"
          class="flex-grow-1 mt-2"
          :label="t('dialog.server.info.url')"
        ></copy-text-input>
        <div style="text-align: center">
          <qr-viewer ref="qrRef" v-model="server_url" width="240px" height="240px"></qr-viewer>
        </div>
        <v-btn :text="t('dialog.server.info.copyQr')" @click="copyQr">
          <template v-slot:prepend>
            <v-fade-transition leave-absolute>
              <v-icon v-if="copied" :icon="mdiCheck"></v-icon>
              <v-icon v-else :icon="mdiContentCopy"></v-icon>
            </v-fade-transition>
          </template>
        </v-btn>
      </v-sheet>
      <v-footer class="flex-grow-0 d-flex align-center ml-0 mr-0 w-100 mt-auto ga-3">
        <v-btn
          :text="t('general.close')"
          class="ml-auto mr-0"
          variant="outlined"
          @click="isServerInfoDialogOpen = false"
        ></v-btn>
      </v-footer>
    </v-dialog>
  </v-dialog>
</template>

<script setup lang="ts">
  import { onMounted, onUnmounted, ref, useTemplateRef, watch } from 'vue';
  import TextInput from '../input/TextInput.vue';
  import { useI18n } from 'vue-i18n';
  import { useApi } from '../../api';
  import { ApiServerOptions } from '../../types/ApiServerOptions';
  import { useUiState } from '../../stores/uistate';
  import { mdiCheck, mdiContentCopy, mdiEye, mdiEyeOff } from '@mdi/js';
  import CopyTextInput from '../input/CopyTextInput.vue';
  import QrViewer from '../input/QrViewer.vue';

  const CHARSET = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';

  const { t } = useI18n();
  const api = useApi();
  const uiState = useUiState();

  const isServerPanelOpen = defineModel<boolean>({ required: true });
  const qrRef = useTemplateRef<InstanceType<typeof QrViewer>>('qrRef');

  const isPasswordVisible = ref(false);
  const isServerInfoDialogOpen = ref(false);
  const copied = ref(false);

  const isRunning = ref<boolean | null>(null);
  const isDiscoverable = ref<boolean | null>(null);
  const server_port = ref<string>('');
  const server_name = ref<string>('Untitled SBS Player Server');
  const server_password = ref<string>('');

  const server_hostname = ref<string | null>(null);
  const server_url = ref<string | null>(null);

  let server_options: ApiServerOptions = {
    port: 5800,
    discoverry: null,
    password: null,
  };

  let unlisten: (() => void) | null = null;

  const saveServerOptions = async () => {
    if (isDiscoverable.value) {
      server_options.discoverry = server_name.value;
    } else {
      server_options.discoverry = null;
    }
    server_options.port = parseInt(server_port.value);
    let new_password: string | null = server_password.value.trim();
    if (new_password === '') {
      new_password = null;
    }
    server_options.password = new_password;
    await api.host?.setServerOptions(server_options);
  };

  const toggleServer = async () => {
    if (isRunning.value) {
      api.host?.stopServer().catch((e) => {
        console.error(e);
        uiState.error_messages.push(e);
      });
    } else {
      isPasswordVisible.value = false;
      await saveServerOptions();
      api.host?.startServer().catch((e) => {
        console.error(e);
        uiState.error_messages.push(e);
      });
    }
  };

  const generateRandomPassword = (): string => {
    const array = new Uint32Array(16);
    crypto.getRandomValues(array);

    let password = '';
    for (let i = 0; i < 16; i++) {
      password += CHARSET[array[i]! % CHARSET.length];
    }

    return password;
  };

  const generateServerUrl = async () => {
    const hostname = await api.host?.getHostname();
    server_hostname.value = hostname || null;
    if (hostname) {
      const address = `${hostname}:${server_options.port}`;
      if (server_options.password) {
        server_url.value = `http://${address}/?address=${address}#${server_options.password}`;
      } else {
        server_url.value = `http://${address}/?address=${address}#`;
      }
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
        server_password.value = options.password || '';
      })
      .catch((e) => console.error(e));
    api.host
      ?.isServerRunning()
      .then((state) => (isRunning.value = state))
      .catch((e) => console.error(e));
    api.host
      ?.onServerStatusChanged((status) => {
        if (status == 'started') {
          isRunning.value = true;
        } else if (status == 'stopped') {
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
