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
        :class="dragOverIndex == i ? $style['drag-over-row'] : ''"
        draggable="true"
        @dragstart="dragStart($event, cue.id)"
        @dragover="dragOver($event, i)"
        @dragend="dragEnd"
        @drop="drop($event, i)"
      >
        <td width="24px">
          <v-icon v-if="i === 1" :icon="mdiArrowRightBold"></v-icon>
        </td>
        <td width="24px">
          <v-icon :icon="mdiVolumeHigh" />
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
  mdiVolumeHigh,
} from "@mdi/js";

const showModelState = useShowModel();

const showModel = showModelState.model;

const dragOverIndex = ref();

const dragStart = (event: DragEvent, cue_id: string) => {
  if (event.dataTransfer) {
    event.dataTransfer.effectAllowed = "move";
    event.dataTransfer.dropEffect = "move";
    event.dataTransfer.setData("text/plain", cue_id);
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
    const cue_id = event.dataTransfer.getData("text/plain");
    // invoke("move_cue", {cue_id: cue_id, to_index: index});
    showModelState.moveCue(cue_id, index);
  }
};
</script>

<style lang="css" module>
.drag-over-row {
  border-top: 4px solid rgb(var(--v-theme-primary)) !important;
}
</style>
