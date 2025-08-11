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
        :class="[dragOverIndex == i ? $style['drag-over-row'] : '', uiState.selectedRows.includes(i) ? $style['selected-row'] : '']"
        draggable="true"
        @dragstart="dragStart($event, i)"
        @dragover="dragOver($event, i)"
        @dragend="dragEnd"
        @drop="drop($event, i)"
        @click="click($event, i)"
      >
        <td width="24px">
          <v-icon :icon="showState.playbackCursor == cue.id ? mdiArrowRightBold : undefined"></v-icon>
        </td>
        <td width="24px">
          <v-icon :icon="getCueIcon(cue.params.type)" />
        </td>
        <td class="text-center" width="50px">
          <span class="cue-number mr-2">{{ cue.number }}</span>
        </td>
        <td width="auto">         {{ cue.name }}
        </td>
        <td class="text-center pa-1" width="100px">
          <div
            :class="[cue.id in showState.activeCues && showState.activeCues[cue.id]!.status == 'PreWaiting' ? 'border-md border-primary' : '']"
            :style="{
              background:
                cue.id in showState.activeCues && showState.activeCues[cue.id]!.status == 'PreWaiting' ?
                'linear-gradient(to right, rgba(var(--v-theme-primary), 0.5) ' +
                Math.floor(showState.activeCues[cue.id]!.position * 100 / showState.activeCues[cue.id]!.duration) +
                '%, transparent ' +
                Math.floor(showState.activeCues[cue.id]!.position * 100 / showState.activeCues[cue.id]!.duration) +
                '%)' : '',
              backgroundRepeat: 'no-repeat',
            }"
          >
            {{ cue.id in showState.activeCues && showState.activeCues[cue.id]!.status == 'PreWaiting' ? secondsToFormat(showState.activeCues[cue.id]!.position) : secondsToFormat(cue.preWait) }}
          </div>
        </td>
        <td class="text-center pa-1" width="100px">
          <div
            :class="[cue.id in showState.activeCues && showState.activeCues[cue.id]!.status == 'Playing' ? 'border-md border-primary' : '']"
            :style="{
              background:
                cue.id in showState.activeCues && showState.activeCues[cue.id]!.status == 'Playing' ?
                'linear-gradient(to right, rgba(var(--v-theme-primary), 0.5) ' +
                Math.floor(showState.activeCues[cue.id]!.position * 100 / showState.activeCues[cue.id]!.duration) +
                '%, transparent ' +
                Math.floor(showState.activeCues[cue.id]!.position * 100 / showState.activeCues[cue.id]!.duration) +
                '%)' : '',
              backgroundRepeat: 'no-repeat',
            }"
          >
           {{ cue.id in showState.activeCues && showState.activeCues[cue.id]!.status == 'Playing' ? secondsToFormat(showState.activeCues[cue.id]!.position) : "05:00.00" }}
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
            {{ cue.sequence.type == "autoFollow" ? secondsToFormat(cue.sequence.postWait) : '00:00.00' }}
          </div>
        </td>
        <td width="24px">
          <v-icon v-if="cue.sequence.type != 'autoFollow'" :icon="mdiArrowBottomLeft" />
        </td>
      </tr>
      <tr></tr>
    </tbody>
  </v-table>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { useShowModel } from "../stores/showmodel";
import {
  mdiArrowBottomLeft,
  mdiArrowRightBold,
  mdiChevronDoubleDown,
  mdiTimerSandEmpty,
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

const click = (event: MouseEvent, index: number) => {
    if (event.shiftKey) {
      if (uiState.selected != null) {
          uiState.selectedRows = [];
          if (index >= uiState.selected) {
            for (let i = uiState.selected; i <= index; i++) {
              uiState.selectedRows.push(i);
            }
          } else {
            for (let i = index; i <= uiState.selected; i++) {
              uiState.selectedRows.push(i);
            }
          }
      } else {
        uiState.selectedRows = [index];
      }
      uiState.selected = index;
    } else if (event.ctrlKey) {
      if (uiState.selected != null) {
        if (index in uiState.selectedRows) {
          uiState.selectedRows.splice(uiState.selectedRows.findIndex((row) => row === index), 1);
          if (uiState.selectedRows.length === 0) {
            uiState.selected = null;
          } else if (index === showModel.cues.findIndex(cue => cue.id == showState.playbackCursor)) {
            uiState.selected = uiState.selectedRows.reduce((a,b) => Math.max(a,b));
          }
        } else {
          uiState.selectedRows.push(index);
          uiState.selected = index;
        }
      } else {
        uiState.selectedRows = [index];
        uiState.selected = index;
      }
    } else {
      uiState.selectedRows = [index];
      uiState.selected = index;
    }
    if (uiSettings.lockCursorToSelection) {
        invoke("set_playback_cursor", {
          cueId: uiState.selected !== null ? showModel.cues[uiState.selected].id : null
        }).catch((e) => {
            console.error("Failed to set cursor. " + e);
        })
    }
}

const getCueIcon = (type: string): string|undefined => {
  switch (type) {
    case "audio":
      return mdiVolumeHigh;
    case "wait":
      return mdiTimerSandEmpty;
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
