<template>
  <v-table fixed-header density="compact" class="flex-grow-1" height="0">
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
          <v-icon v-if="i === 1" :icon="mdiArrowRightBold"></v-icon>
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
            {{ Duration.fromMillis(cue.preWait * 1000).toFormat("mm:ss.SS")}}
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
            {{ Duration.fromMillis(cue.postWait * 1000).toFormat("mm:ss.SS")}}
          </div>
        </td>
        <td width="24px">
          <v-icon :icon="mdiArrowBottomLeft" />
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
  mdiArrowRightBold,
  mdiChevronDoubleDown,
  mdiVolumeHigh,
} from "@mdi/js";
import { Duration } from "luxon";
import { useUiState } from "../stores/uistate";

const showModel = useShowModel();
const uiState = useUiState();

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
    const cue_id = showModel.cues[fromIndex].id;
    const newIndex = index < fromIndex ? index : index - 1;
    // invoke("move_cue", {cue_id: cue_id, to_index: index});
    showModel.moveCue(cue_id, newIndex);
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
            uiState.selected = index;
        }
    } else {
        uiState.selectedRange = null;
        uiState.selected = index;
    }
}
</script>

<style lang="css" module>
.drag-over-row > td {
  border-top: 2px solid rgb(var(--v-theme-primary)) !important;
}
.selected-row > td {
    background-color: rgb(var(--v-theme-primary));
    color: rgb(var(--v-theme-on-primary));
}
</style>
