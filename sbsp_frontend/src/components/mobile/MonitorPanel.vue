<template>
  <v-sheet class="h-100 pa-3">
    <div class="text-title-medium mb-2">
      {{ t('main.sideBar.activeCues') }}
    </div>
    <template
      v-for="(activeCue, cue_id) in showState.activeCues"
      :key="cue_id"
    >
      <active-cue-item :active-cue="activeCue" />
    </template>
  </v-sheet>
</template>

<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { useShowState } from '../../stores/showstate';
import { useI18n } from 'vue-i18n';
import ActiveCueItem from '../ActiveCueItem.vue';

const showState = useShowState();
const { t } = useI18n();
</script>
