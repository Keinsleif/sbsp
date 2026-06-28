<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { mdiOpenInNew, mdiTrashCan } from '@mdi/js';
import { generateRandomPassword } from '../../utils';
import { useI18n } from 'vue-i18n';
import CopyTextInput from '../input/CopyTextInput.vue';
import PermissionSelect from '../input/PermissionSelect.vue';
import ButtonWrapper from '../wrapper/ButtonWrapper.vue';

const { t } = useI18n();

const password = defineModel<string>('password', { required: true });
const permission = defineModel<number>('permission', { required: true });

const props = defineProps<{
  isRunning: boolean;
  isVisible: boolean;
}>();
const emit = defineEmits(['delete', 'openInfo']);

</script>

<template>
  <tr>
    <td class="p-0">
      <copy-text-input
        v-model="password"
        :placeholder="t('dialog.server.info.passwordNotSet')"
        :type="props.isVisible ? 'text' : 'password'"
      />
    </td>
    <td class="p-0">
      <permission-select v-model="permission" class="w-full" />
    </td>
    <td class="p-2">
      <button-wrapper
        class="w-full"
        :disabled="props.isRunning"
        :label="t('dialog.server.generate')"
        severity="primary"
        @click="password = generateRandomPassword()"
      />
    </td>
    <td class="p-2">
      <button-wrapper
        variant="outlined"
        :icon="mdiOpenInNew"
        @click="emit('openInfo')"
      />
    </td>
    <td class="p-2">
      <button-wrapper
        variant="outlined"
        :disabled="props.isRunning"
        :icon="mdiTrashCan"
        severity="danger"
        @click="emit('delete')"
      />
    </td>
  </tr>
</template>
