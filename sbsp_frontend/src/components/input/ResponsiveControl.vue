<template>
  <template v-if="!controlProps.overlay">
    <slot />
  </template>
  <v-dialog v-else>
    <template #default="{ isActive }">
      <v-card>
        <v-card-text>
          <slot />
        </v-card-text>

        <v-card-actions>
          <v-spacer />
          <v-btn
            variant="outlined"
            @click="isActive.value = false"
          >
            {{ t('general.close') }}
          </v-btn>
        </v-card-actions>
      </v-card>
    </template>
    <template #activator="{ props }">
      <v-btn
        v-bind="props"
        variant="outlined"
      >
        {{ controlProps.buttonLabel }}
      </v-btn>
    </template>
  </v-dialog>
</template>

<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { useI18n } from 'vue-i18n';

const { t } = useI18n();

const controlProps = withDefaults(
  defineProps<{
    overlay?: boolean;
    buttonLabel?: string;
  }>(),
  {
    overlay: false,
    buttonLabel: 'Open',
  },
);
</script>
