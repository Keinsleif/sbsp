<template>
  <v-table fixed-header density="compact" class="flex-grow-1" height="100%">
    <thead>
      <tr>
        <th></th>
        <th></th>
        <th class="text-center">Number</th>
        <th>Name</th>
        <th class="text-center">Pre-Wait</th>
        <th class="text-center">Duration</th>
        <th class="text-center">Post-Wait</th>
        <th><v-icon :icon="mdiChevronDoubleDown" /></th>
      </tr>
    </thead>
    <tbody>
      <tr
        v-for="(cue, i) in showModel.cues"
        :key="cue.id"
        :class="[dragOverIndex == i ? $style['drag-over-row'] : '', isSelected(i) ? $style['selected-row'] : '']"
        draggable="true"
        @dragstart="dragStart($event, i)"
        @dragover="dragOver($event, i)"
        @dragend="dragEnd"
        @drop="drop($event, i)"
        @click="click($event, i)"
      >
        <td width="24px">
          <v-icon v-if="showState.playbackCursor == cue.id" :icon="mdiArrowRightBold"></v-icon>
        </td>
        <td width="24px">
          <v-icon v-if="cue.param.type == 'audio'" :icon="mdiVolumeHigh" />
        </td>
        <td class="text-center" width="50px">
          <span class="cue-number mr-2">{{ cue.number }}.0</span>
        </td>
        <td width="auto">
          {{ cue.name }}
        </td>
        <td class="text-center pa-1" width="100px">
          <div
            class="border-md border-primary"
            :style="{
              background:
                'linear-gradient(to right, rgba(var(--v-theme-primary), 0.5) ' +
                2 * i +
                '%, transparent ' +
                2 * i +
                '%)',
              backgroundRepeat: 'no-repeat',
            }"
          >
            {{ secondsToFormat(cue.preWait) }}
          </div>
        </td>
        <td class="text-center pa-1" width="100px">
          <div
            class="border-md border-primary"
            :style="{
              background:
                'linear-gradient(to right, rgba(var(--v-theme-primary), 0.5) ' +
                2 * i +
                '%, transparent ' +
                2 * i +
                '%)',
              backgroundRepeat: 'no-repeat',
            }"
          >
            05:00.00
          </div>
        </td>
        <td class="text-center pa-1" width="100px">
          <div
            class="border-md border-primary"
            :style="{
              background:
                'linear-gradient(to right, rgba(var(--v-theme-primary), 0.5) ' +
                2 * i +
                '%, transparent ' +
                2 * i +
                '%)',
              backgroundRepeat: 'no-repeat',
            }"
          >
            {{ secondsToFormat(cue.postWait) }}
          </div>
        </td>
        <td width="24px">
          <v-icon v-if="cue.sequence == 'autoContinue'" :icon="mdiArrowBottomLeft" />
          <v-icon v-if="cue.sequence == 'autoFollow'" :icon="mdiArrowDown" />
        </td>
      </tr>
      <tr></tr>
    </tbody>
  </v-table>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { useShowModel } from "../stores/showmodel";
import {
  mdiArrowBottomLeft,
  mdiArrowDown,
  mdiArrowRightBold,
  mdiChevronDoubleDown,
  mdiVolumeHigh,
} from "@mdi/js";
import { useUiState } from "../stores/uistate";
import { useShowState } from "../stores/showstate";
import { invoke } from "@tauri-apps/api/core";
import { useUiSettings } from "../stores/uisettings";
import { secondsToFormat } from "../utils";

const showModel = useShowModel();
const showState = useShowState();
const uiState = useUiState();
const uiSettings = useUiSettings();

const dragOverIndex = ref();

const dragStart = (event: DragEvent, index: number) => {
  if (event.dataTransfer) {
    event.dataTransfer.effectAllowed = "move";
    event.dataTransfer.dropEffect = "move";
    event.dataTransfer.setData("text/plain", index.toString());
  }
};

const dragOver = (event: DragEvent, index: number) => {
  event.preventDefault();
  dragOverIndex.value = index;
};

const dragEnd = () => {
    dragOverIndex.value = null;
}

const drop = (event: DragEvent, index: number) => {
  event.preventDefault();
  if (event.dataTransfer) {
    const fromIndex = Number(event.dataTransfer.getData("text/plain"));
    const cueId = showModel.cues[fromIndex].id;
    const newIndex = index < fromIndex ? index : index - 1;
    invoke("move_cue", {cueId: cueId, to_index: newIndex}).catch((e)=>{
        console.log("Failed to move cue. "+e);
    });
    // showModel.moveCue(cue_id, newIndex);
  }
};

const isSelected = computed(() => {
    return (index: number) => {
        if (uiState.selectedRange == null) {
            return uiState.selected == index;
        } else {
            return uiState.selectedRange[0] <= index && uiState.selectedRange[1] >= index;
        }
    }
})

const click = (event: MouseEvent, index: number) => {
    if (event.shiftKey) {
        if (uiState.selected != null) {
            if (index >= uiState.selected) {
                uiState.selectedRange = [uiState.selected, index];
            } else {
                uiState.selectedRange = [index, uiState.selected];
            }
        } else {
            uiState.selectedRange = null;
        }
    } else {
        uiState.selectedRange = null;
    }
    uiState.selected = index;
    if (uiSettings.lockCursorToSelection) {
        invoke("set_playback_cursor", {cueId: showModel.cues[index].id}).catch((e) => {
            console.error("Failed to set cursor. " + e);
        })
    }
}
</script>

<style lang="css" module>
.drag-over-row > td {
  border-top: 2px solid rgb(var(--v-theme-primary)) !important;
}
.selected-row > td {
    background-color: rgb(var(--v-theme-primary), 0.2);
    color: rgb(var(--v-theme-on-background));
}
</style>
