<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { useShowState } from '../../stores/showState';
import { useI18n } from 'vue-i18n';
import ActiveCueItem from '../ActiveCueItem.vue';

const showState = useShowState();
const { t } = useI18n();
</script>

<template>
  <div class="h-full p-3 flex flex-col gap-2">
    <h2>
      {{ t('main.sideBar.activeCues') }}
    </h2>
    <div class="border-t border-x border-(--p-form-field-border-color) overflow-auto">
      <template
        v-for="(activeCue, cue_id) in showState.activeCues"
        :key="cue_id"
      >
        <active-cue-item :active-cue="activeCue" />
      </template>
    </div>
  </div>
</template>
