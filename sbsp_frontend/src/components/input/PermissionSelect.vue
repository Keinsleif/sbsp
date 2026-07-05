<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import MultiSelect from 'primevue/multiselect';
import { computed, ref, watch } from 'vue';
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

const options = computed(() => {
  return [
    { title: t('dialog.server.permissions.read'), value: 1 },
    { title: t('dialog.server.permissions.control'), value: 2 },
    { title: t('dialog.server.permissions.edit'), value: 4 },
  ];
})
</script>

<template>
  <multi-select
    v-model="innerPerm"
    :options="options"
    option-label="title"
    option-value="value"
    display="chip"
    :show-toggle-all="false"
    autocomplete="off"
    @update:model-value="save"
  />
</template>
