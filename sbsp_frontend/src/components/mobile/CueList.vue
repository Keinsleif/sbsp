<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { mdiChevronDoubleDown, mdiRepeat } from '@mdi/js';
import { useI18n } from 'vue-i18n';
import { useShowModel } from '../../stores/showModel';
import CueListRow from './CueListRow.vue';
import { getLockCursorToSelection } from '../../utils';
import { useApi } from '../../api';
import { useUiState } from '../../stores/uiState';
import PathIcon from '../display/PathIcon.vue';

const { t } = useI18n();
const api = useApi();
const showModel = useShowModel();
const uiState = useUiState();

const setPlaybackCursor = (cueId: string) => {
  if (uiState.mode !== 'view' && getLockCursorToSelection()) {
    api.setPlaybackCursor(cueId);
  }
};
</script>

<template>
  <div class="grow border border-(--p-form-field-border-color) overflow-auto w-full">
    <table :class="$style['cuelist']" class="w-full table-fixed ">
      <thead>
        <tr>
          <th
            id="cuelist_cursor"
            width="20px"
            class="px-1"
          />
          <th
            id="cuelist_status"
            width="20px"
            class="px-1"
          />
          <th
            id="cuelist_type"
            width="24px"
            class="px-1"
          />
          <th
            id="cuelist_number"
            class="text-center border-l border-(--p-form-field-border-color) px-1"
            width="32px"
          >
            #
          </th>
          <th
            id="cuelist_name"
            class="border-x border-(--p-form-field-border-color) overflow-hidden whitespace-nowrap w-fit"
            style="padding-left: 24px"
          >
            {{ t('main.name') }}
          </th>
          <th
            id="cuelist_repeat"
            width="20px"
            class="border-r border-(--p-form-field-border-color) px-0"
          >
            <path-icon :icon="mdiRepeat" />
          </th>
          <th
            id="cuelist_chain"
            width="20px"
            class="px-0"
          >
            <path-icon :icon="mdiChevronDoubleDown" />
          </th>
        </tr>
      </thead>
      <tbody>
        <cue-list-row
          v-for="(item) in showModel.flatCueList"
          v-show="!item.isHidden"
          ref="cuelistItem"
          :key="item.cue.id"
          :item="item"
          @pointerdown="setPlaybackCursor(item.cue.id)"
        />
        <tr>
          <td colspan="7" />
        </tr>
      </tbody>
    </table>
  </div>
</template>

<style lang="css" module>
.cuelist {
  scroll-padding-top: 32px;
  font-size: 0.8em;
}

.cuelist th {
  position: sticky;
  background-color: color-mix(in oklab, var(--p-content-background) 60%, var(--p-surface-500));
  top: 0;
  z-index: 10;
}

@layer base {
  .cuelist th,
  .cuelist td {
    height: 32px;
    text-align: left;
    border-bottom: 1px solid var(--p-form-field-border-color);
  }
}
  /* .cuelist {
    table {
      table-layout: fixed;
      font-size: 0.9em;
    }
    > div {
      scroll-padding-top: 34px;
    }
  } */
</style>
