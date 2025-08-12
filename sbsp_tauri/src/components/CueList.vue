<template>
  <v-table fixed-header density="compact" class="flex-grow-1" height="100%">
    <thead>
      <tr>
        <th id="cuelist_cursor"></th>
        <th id="cuelist_type"></th>
        <th id="cuelist_number" class="text-center">Number</th>
        <th id="cuelist_name">Name</th>
        <th id="cuelist_pre_wait" class="text-center">Pre-Wait</th>
        <th id="cuelist_duration" class="text-center">Duration</th>
        <th id="cuelist_post_wait" class="text-center">Post-Wait</th>
        <th id="cuelist_sequence"><v-icon :icon="mdiChevronDoubleDown" /></th>
      </tr>
    </thead>
    <tbody>
      <tr
        v-for="(cue, i) in showModel.cues"
        :key="cue.id"
        :class="[
          dragOverIndex == i ? $style['drag-over-row'] : '',
          uiState.selectedRows.includes(i) ? $style['selected-row'] : '',
        ]"
        draggable="true"
        @dragstart="dragStart($event, i)"
        @dragover="dragOver($event, i)"
        @dragend="dragEnd"
        @drop="drop($event, i)"
        @click="click($event, i)"
      >
        <td headers="cuelist_cursor" width="24px">
          <v-icon
            :icon="
              showState.playbackCursor == cue.id ? mdiArrowRightBold : undefined
            "
          ></v-icon>
        </td>
        <td headers="cuelist_type" width="24px">
          <v-icon :icon="getCueIcon(cue.params.type)" />
        </td>
        <td headers="cuelist_number" class="text-center" width="50px" @dblclick="openEditable($event)" @blur="closeEditable($event.target, true, i)" @keydown.enter="closeEditable($event.target, true, i)" @keydown.esc="closeEditable($event.target, false, i)">
          <span class="cue-number mr-2">{{ cue.number }}</span>
        </td>
        <td headers="cuelist_name" width="auto" @dblclick="openEditable($event)" @blur="closeEditable($event.target, true, i)" @keydown.enter="closeEditable($event.target, true, i)" @keydown.esc="closeEditable($event.target, false, i)">{{ cue.name }}</td>
        <td headers="cuelist_pre_wait" class="text-center pa-1" width="100px" @dblclick="openEditable($event)" @blur="closeEditable($event.target, true, i)" @keydown.enter="closeEditable($event.target, true, i)" @keydown.esc="closeEditable($event.target, false, i)">
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
              pointerEvents: 'none',
            }"
          >
            {{
              cue.id in showState.activeCues &&
              showState.activeCues[cue.id]!.status == "PreWaiting"
                ? secondsToFormat(showState.activeCues[cue.id]!.position)
                : secondsToFormat(cue.preWait)
            }}
          </div>
        </td>
        <td headers="cuelist_duration" class="text-center pa-1" width="100px" @dblclick="openEditable($event)" @blur="closeEditable($event.target, true, i)" @keydown.enter="closeEditable($event.target, true, i)" @keydown.esc="closeEditable($event.target, false, i)">
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
              pointerEvents: 'none',
            }"
          >
            {{
              cue.id in showState.activeCues &&
              showState.activeCues[cue.id]!.status == "Playing"
                ? secondsToFormat(showState.activeCues[cue.id]!.position)
                : "05:00.00"
            }}
          </div>
        </td>
        <td headers="cuelist_post_wait" class="text-center pa-1" width="100px" @dblclick="openEditable($event)" @blur="closeEditable($event.target, true, i)" @keydown.enter="closeEditable($event.target, true, i)" @keydown.esc="closeEditable($event.target, false, i)">
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
              pointerEvents: 'none',
            }"
          >
            {{
              cue.sequence.type == "autoFollow"
                ? secondsToFormat(cue.sequence.postWait)
                : "00:00.00"
            }}
          </div>
        </td>
        <td headers="cuelist_sequence" width="24px">
          <v-icon
            v-if="cue.sequence.type != 'autoFollow'"
            :icon="mdiArrowBottomLeft"
          />
        </td>
      </tr>
      <tr
        :class="
          dragOverIndex == showModel.cues.length ? $style['drag-over-row'] : ''
        "
        @dragover="dragOver($event, showModel.cues.length)"
        @drop="drop($event, showModel.cues.length)"
      >
        <td headers="cuelist_cursor"></td>
        <td headers="cuelist_type"></td>
        <td headers="cuelist_number"></td>
        <td headers="cuelist_name"></td>
        <td headers="cuelist_pre_wait"></td>
        <td headers="cuelist_duration"></td>
        <td headers="cuelist_post_wait"></td>
        <td headers="cuelist_sequence"></td>
      </tr>
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
import { Cue } from "../types/Cue";

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
};

const drop = (event: DragEvent, index: number) => {
  event.preventDefault();
  if (event.dataTransfer) {
    const fromIndex = Number(event.dataTransfer.getData("text/plain"));
    const cueId = showModel.cues[fromIndex].id;
    if (fromIndex === index) {
      return;
    }
    const newIndex = index < fromIndex ? index : index - 1;
    invoke("move_cue", { cueId: cueId, toIndex: newIndex }).catch((e) => {
      console.log("Failed to move cue. " + e);
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
        uiState.selectedRows.splice(
          uiState.selectedRows.findIndex((row) => row === index),
          1
        );
        if (uiState.selectedRows.length === 0) {
          uiState.selected = null;
        } else if (
          index ===
          showModel.cues.findIndex((cue) => cue.id == showState.playbackCursor)
        ) {
          uiState.selected = uiState.selectedRows.reduce((a, b) =>
            Math.max(a, b)
          );
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
      cueId:
        uiState.selected !== null ? showModel.cues[uiState.selected].id : null,
    }).catch((e) => {
      console.error("Failed to set cursor. " + e);
    });
  }
};

const getCueIcon = (type: string): string | undefined => {
  switch (type) {
    case "audio":
      return mdiVolumeHigh;
    case "wait":
      return mdiTimerSandEmpty;
  }
};

const openEditable = (e: MouseEvent) => {
  if (e.target == null || !(e.target instanceof HTMLTableCellElement) || e.target.contentEditable === "true") {
    return;
  }
  e.target.contentEditable="true";
  e.target.classList.add('inEdit');
  e.target.dataset.prevText = e.target.innerText;
  var range = document.createRange();
  range.selectNodeContents(e.target);
  var sel = window.getSelection();
  if (sel != null) {
    sel.removeAllRanges();
    sel.addRange(range);
  } else {
    e.target.focus();
  }
}

const closeEditable = (target: EventTarget|null, needSave: boolean, rowIndex: number) => {
  if (target == null || !(target instanceof HTMLTableCellElement) || target.contentEditable === "false") {
    return;
  }
  target.contentEditable="false";
  target.classList.remove('inEdit');
  if (needSave) {
    const newCue = JSON.parse(JSON.stringify(showModel.cues[rowIndex])) as Cue;
    switch (target.headers) {
      case "cuelist_number":
        newCue.number = target.innerText;
        break;
      case "cuelist_name":
        newCue.name = target.innerText;
        break;
      case "cuelist_pre_wait": {
        let newPreWait = Number(target.innerText);
        if (isNaN(newPreWait) || newPreWait < 0) {
          newPreWait = 0;
        }
        newCue.preWait = newPreWait;
        break;
      }
      case "cuelist_duration": {
        if (newCue.params.type != "audio") {
          let newDuration = Number(target.innerText);
          if (isNaN(newDuration) || newDuration < 0) {
            newDuration = 0;
          }
          newCue.params.duration = newDuration;
        }
        break;
      }
      case "cuelist_post_wait": {
        if (newCue.sequence.type == "autoFollow") {
          let newPostWait = Number(target.innerText);
          if (isNaN(newPostWait)) {
            newPostWait = 0;
          }
          newCue.sequence.postWait = newPostWait;
        }
        break;
      }
    }
    invoke("update_cue", {cue: newCue}).catch(e=>console.log(e.toString()));
  } else {
    if (target.dataset.prevText != undefined) {
      target.innerText = target.dataset.prevText;
    }
  }
  target.dataset.prevText = undefined;
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
