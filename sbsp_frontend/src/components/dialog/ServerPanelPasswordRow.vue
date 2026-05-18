<template>
  <tr>
    <td class="pa-0">
      <copy-text-input
        :placeholder="t('dialog.server.info.passwordNotSet')"
        :type="props.isVisible ? 'text' : 'password'"
        align-input="left"
        v-model="password"
      />
    </td>
    <td class="pa-0">
      <permission-select v-model="permission" />
    </td>
    <td class="pa-2">
      <v-btn
        width="100%"
        :disabled="props.isRunning || false"
        :text="t('dialog.server.generate')"
        color="primary"
        density="compact"
        @click="password = generateRandomPassword()"
      />
    </td>
    <td class="pa-2">
      <v-btn
        density="compact"
        :icon="mdiOpenInNew"
        @click="emit('openInfo')"
      />
    </td>
    <td class="pa-2">
      <v-btn
        density="compact"
        :disabled="props.isRunning || false"
        :icon="mdiTrashCan"
        color="error"
        @click="emit('delete')"
      />
    </td>
  </tr>
</template>

<script setup lang="ts">
import { mdiOpenInNew, mdiTrashCan } from '@mdi/js';
import { generateRandomPassword } from '../../utils';
import { useI18n } from 'vue-i18n';
import CopyTextInput from '../input/CopyTextInput.vue';
import PermissionSelect from '../input/PermissionSelect.vue';

const { t } = useI18n();

const password = defineModel<string>('password', { required: true });
const permission = defineModel<number>('permission', { required: true });

const props = defineProps<{
  isRunning: boolean;
  isVisible: boolean;
}>();
const emit = defineEmits(['delete', 'openInfo']);

</script>
