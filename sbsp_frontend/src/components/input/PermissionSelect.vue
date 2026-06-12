<template>
  <v-select
    v-model="innerPerm"
    :items="[
      { title: t('dialog.server.permissions.read'), value: 1},
      { title: t('dialog.server.permissions.control'), value: 2},
      { title: t('dialog.server.permissions.edit'), value: 4},
    ]"
    density="compact"
    variant="outlined"
    chips
    multiple
    autocomplete="off"
    hide-details
    @update:model-value="save"
  />
</template>

<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();
const perm = defineModel<number>({ required: true });

const permToInner = (perm: number): number[] => {
  const permList = [];
  for (let i = 0; i < 4; i++) {
    const value = ((perm >> i) & 1) * Math.pow(2, i);
    if (value !== 0) {
      permList.push(value);
    }
  }
  return permList;
};
const innerPerm = ref<number[]>(permToInner(perm.value));

watch(perm, (newValue, oldValue) => {
  if (newValue === oldValue) return;
  innerPerm.value = permToInner(newValue);
});

const save = () => {
  document.body.focus();
  perm.value = innerPerm.value.reduce((acc, cur) => acc + cur, 0);
};
</script>
