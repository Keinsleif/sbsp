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
      <tr
        v-for="(item) in showModel.flatCueList"
        v-show="!item.isHidden"
        ref="cuelistItem"
        :key="item.cue.id"
        @click="setPlaybackCursor(item.cue.id)"
      >
        <td
          headers="cuelist_cursor"
          class="px-1"
        >
          <v-icon
            :icon="showState.playbackCursor == item.cue.id ? mdiArrowRightBold : undefined"
          />
        </td>
        <td
          headers="cuelist_status"
          class="px-1"
        >
          <v-icon
            v-show="isStatusIn(item.cue.id, ['playing', 'preWaiting'])"
            :icon="mdiPlay"
            color="success"
          />
          <v-icon
            v-show="isStatusIn(item.cue.id, ['paused', 'preWaitPaused'])"
            :icon="mdiPause"
            color="warning"
          />
          <v-icon
            v-show="getStatus(item.cue.id) == 'loaded'"
            :icon="mdiUpload"
            color="warning"
          />
          <v-progress-circular
            v-show="getStatus(item.cue.id) == 'stopping'"
            indeterminate="disable-shrink"
            size="16"
            color="warning"
          />
        </td>
        <td
          headers="cuelist_type"
          class="text-center"
          style="padding: 0px"
        >
          <v-icon :icon="getCueIcon(item.cue.params.type)" />
        </td>
        <td
          headers="cuelist_number"
          class="text-center border-e border-s px-1"
        >
          {{ item.cue.number }}
        </td>
        <td
          headers="cuelist_name"
          class="border-e overflow-hidden text-no-wrap"
          :style="{
            paddingLeft: `${item.level}em`,
          }"
        >
          <v-icon
            :icon="item.isGroup ? (uiState.expandedRows.includes(item.cue.id) ? mdiMenuDown : mdiMenuRight) : undefined"
            @click.stop="if (item.isGroup) uiState.toggleExpand(item.cue.id);"
            @pointerdown.stop
          />
          {{ item.cue.name != null ? item.cue.name : buildCueName(item.cue) }}
        </td>
        <td headers="cuelist_repeat" class="px-0 border-e">
          <v-icon
            v-show="
              (item.cue.params.type == 'audio' && item.cue.params.repeat) ||
                (item.cue.params.type == 'group' &&
                  item.cue.params.mode.type == 'playlist' &&
                  item.cue.params.mode.repeat)
            "
            :icon="mdiRepeat"
          />
        </td>
        <td headers="cuelist_chain" class="px-0">
          <v-icon
            v-show="item.chain.type == 'afterComplete'"
            :icon="mdiArrowCollapseDown"
          />
          <v-icon
            v-show="item.chain.type == 'afterStart'"
            :icon="mdiArrowExpandDown"
          />
        </td>
      </tr>
      <tr>
        <td headers="cuelist_cursor" />
        <td headers="cuelist_status" />
        <td headers="cuelist_type" />
        <td headers="cuelist_number" />
        <td headers="cuelist_name" />
        <td headers="cuelist_repeat" />
        <td headers="cuelist_chain" />
      </tr>
    </tbody>
  </v-table>
</template>

<script setup lang="ts">
import { mdiArrowCollapseDown, mdiArrowExpandDown, mdiArrowRightBold, mdiChevronDoubleDown, mdiMenuDown, mdiMenuRight, mdiPause, mdiPlay, mdiRepeat, mdiUpload } from '@mdi/js';
import { useI18n } from 'vue-i18n';
import { useShowModel } from '../../stores/showmodel';
import { useShowState } from '../../stores/showstate';
import type { PlaybackStatus } from '../../types/PlaybackStatus';
import { buildCueName, getCueIcon, getLockCursorToSelection } from '../../utils';
import { useApi } from '../../api';
import { computed } from 'vue';
import { useUiState } from '../../stores/uistate';

const api = useApi();
const { t } = useI18n();
const showModel = useShowModel();
const showState = useShowState();
const uiState = useUiState();

const getStatus = computed(() => (id: string): PlaybackStatus | null => {
  if (showState.activeCues[id] == undefined) {
    return null;
  }
  return showState.activeCues[id].status;
});

const isStatusIn = computed(() => (cueId: string, statusList: PlaybackStatus[]): boolean => {
  const status = getStatus.value(cueId);
  if (status) {
    return statusList.includes(status);
  }
  return false;
});

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
