<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { mdiAlert, mdiClose } from '@mdi/js';
import ButtonWrapper from '../wrapper/ButtonWrapper.vue';
import InputGroup from 'primevue/inputgroup';
import InputGroupAddon from 'primevue/inputgroupaddon';
import { computed, ref, useId } from 'vue';
import FloatLabel from 'primevue/floatlabel';
import InputText from 'primevue/inputtext';
import { MOD_KEY, MODIFIER_KEYS } from '@/composables/useHotkey.ts';
import { useApi } from '@/api/index.ts';
import { normalizeHotkey } from '@/utils.ts';
import PathIcon from '../display/PathIcon.vue';
import { useI18n } from 'vue-i18n';

const api = useApi();
const { t } = useI18n();
const hotkey = defineModel<string | null>({ default: '' });
const hotkeyPreview = ref('');
const hotkeyDisplay = computed(() => {
  if (hotkeyPreview.value) {
    return hotkeyPreview.value.replace('$mod', MOD_KEY).replace('Control', 'Ctrl');
  }
  return hotkey.value != null ? hotkey.value.replace('$mod', MOD_KEY).replace('Control', 'Ctrl') : '';
});
const props = defineProps<{
  label?: string;
  hotkeySet: string[],
}>();
const isDuplicate = computed(() => {
  const currentHotkey = hotkey.value;
  if (currentHotkey == null) return false;
  const count = props.hotkeySet.filter((e) => normalizeHotkey(currentHotkey) === e).length;
  return count > 1;
});

const keydown = (event: KeyboardEvent) => {
  event.preventDefault();
  let shortcut = '';
  if (event.ctrlKey) {
    shortcut += api.isMacOs() ? 'Control+' : '$mod+';
  }
  if (event.metaKey) {
    shortcut += api.isMacOs() ? '$mod+' : 'Meta+';
  }
  if (event.altKey) {
    shortcut += 'Alt+';
  }
  if (event.shiftKey) {
    shortcut += 'Shift+';
  }

  if (MODIFIER_KEYS.has(event.key)) {
    hotkeyPreview.value = shortcut;
    return;
  }

  if (event.key === ' ') {
    shortcut += 'Space';
  } else if (event.key.length === 1) {
    shortcut += event.key.toUpperCase();
  } else {
    shortcut += event.key;
  }
  hotkeyPreview.value = '';
  hotkey.value = shortcut;
};

const keyup = (event: KeyboardEvent) => {
  event.preventDefault();

  if (!hotkeyPreview.value) return;

  let shortcut = '';
  if (event.ctrlKey && event.key !== 'Control') {
    shortcut += api.isMacOs() ? 'Control+' : '$mod+';
  }
  if (event.metaKey && event.key !== 'Meta') {
    shortcut += api.isMacOs() ? '$mod+' : 'Meta+';
  }
  if (event.altKey && event.key !== 'Alt') {
    shortcut += 'Alt+';
  }
  if (event.shiftKey && event.key !== 'Shift') {
    shortcut += 'Shift+';
  }

  hotkeyPreview.value = shortcut;
};

const resetPreview = () => {
  if (hotkeyPreview.value) {
    hotkeyPreview.value = '';
  }
};

const inputId = useId();
</script>

<template>
  <input-group class="w-80">
    <float-label
      variant="on"
    >
      <input-text
        :model-value="hotkeyDisplay"
        class="h-full w-full"
        :id="inputId"
        autocomplete="off"
        :pt="{
          root: () => {
            return {
              style: 'background-color: var(--p-inputtext-background);',
            };
          },
        }"
        @keydown.stop="keydown($event)"
        @keyup.stop="keyup($event)"
        @blur="resetPreview()"
      />
      <label :for="inputId">{{ props.label || '' }}</label>
    </float-label>
    <input-group-addon v-show="isDuplicate" v-tooltip.left="t('dialog.settings.global.hotkey.duplicateWarning')">
      <path-icon class="text-yellow-500" :icon="mdiAlert" />
    </input-group-addon>
    <input-group-addon>
      <button-wrapper
        :icon="mdiClose"
        severity="secondary"
        variant="text"
        @click="hotkey = null"
      />
    </input-group-addon>
  </input-group>
</template>
