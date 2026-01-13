<template>
  <v-dialog
    v-model="isServerPanelOpen"
    width="auto"
    @keydown.esc.stop="isServerPanelOpen = false"
    @keydown.stop
    @contextmenu.prevent
  >
    <v-sheet class="d-flex flex-column w-100 h-100">
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
          @update="savePort"
        ></text-input>
        <v-checkbox
          v-model="isDiscoverable"
          :disabled="isRunning"
          :label="t('dialog.server.discoverable')"
          density="compact"
          hide-details
          @update:model-value="saveDiscoveryOpt"
        ></v-checkbox>
        <text-input
          v-model="server_name"
          :disabled="!isDiscoverable || isRunning"
          align-input="left"
          class="flex-grow-0"
          :label="t('dialog.server.serverName')"
          width="480px"
          @update:model-value="saveDiscoveryOpt"
        ></text-input>
        <v-snackbar-queue v-model="error_messages" timeout="2000" color="error"></v-snackbar-queue>
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
  </v-dialog>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue';
import TextInput from '../input/TextInput.vue';
import { useI18n } from 'vue-i18n';
import { useApi } from '../../api';

const { t } = useI18n();
const api = useApi();

const isServerPanelOpen = defineModel<boolean>({ required: true });

const isRunning = ref<boolean | null>(null);
const server_port = ref<string>('');
const isDiscoverable = ref<boolean | null>(null);
const server_name = ref<string>('Untitled SBS Player Server');
const error_messages = ref<string[]>([]);

let unlisten: (() => void) | null = null;

const toggleServer = () => {
  if (isRunning.value) {
    api.host?.stopServer().catch((e) => {
      console.error(e);
      error_messages.value.push(e);
    });
  } else {
    api.host?.startServer().catch((e) => {
      console.error(e);
      error_messages.value.push(e);
    });
  }
};

const savePort = () => {
  const newPort = parseInt(server_port.value);
  if (!isNaN(newPort)) {
    api.host?.setServerPort(newPort).catch((e) => console.error(e));
  }
};

const saveDiscoveryOpt = () => {
  if (isDiscoverable.value) {
    api.host?.setDiscoveryOption(server_name.value).catch((e) => console.error(e));
  } else {
    api.host?.setDiscoveryOption(null).catch((e) => console.error(e));
  }
};

onMounted(() => {
  api.host
    ?.getServerPort()
    .then((port) => (server_port.value = port.toString()))
    .catch((e) => console.error(e));
  api.host
    ?.getDiscoveryOption()
    .then((name) => {
      if (name != null) {
        isDiscoverable.value = true;
        server_name.value = name;
      } else {
        isDiscoverable.value = false;
      }
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
