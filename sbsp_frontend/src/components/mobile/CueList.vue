<template>
  <v-table
    fixed-header
    density="compact"
    class="flex-grow-1"
    :class="$style['cuelist']"
    height="100%"
  >
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
          class="text-center border-s px-1"
          width="32px"
        >
          #
        </th>
        <th
          id="cuelist_name"
          class="border overflow-hidden text-no-wrap"
          style="padding-left: 24px"
        >
          {{ t('main.name') }}
        </th>
        <th
          id="cuelist_repeat"
          width="20px"
          class="border-e px-0"
        >
          <v-icon :icon="mdiRepeat" />
        </th>
        <th
          id="cuelist_chain"
          width="20px"
          class="px-0"
        >
          <v-icon :icon="mdiChevronDoubleDown" />
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
  </v-table>
</template>

<script setup lang="ts">
import { mdiChevronDoubleDown, mdiRepeat } from '@mdi/js';
import { useI18n } from 'vue-i18n';
import { useShowModel } from '../../stores/showmodel';
import CueListRow from './CueListRow.vue';
import { getLockCursorToSelection } from '../../utils';
import { useApi } from '../../api';

const { t } = useI18n();
const api = useApi();
const showModel = useShowModel();

const setPlaybackCursor = (cueId: string) => {
  if (!getLockCursorToSelection()) {
    api.setPlaybackCursor(cueId);
  }
};
</script>

<style lang="css" module>
  .cuelist {
    table {
      table-layout: fixed;
      font-size: 0.9em;
    }
    > div {
      scroll-padding-top: 34px;
    }
  }
</style>
