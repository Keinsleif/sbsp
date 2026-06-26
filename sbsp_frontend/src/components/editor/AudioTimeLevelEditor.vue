<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { ref, watch } from 'vue';
import VolumeFader from '../input/VolumeFader.vue';
import PanningFader from '../input/PanningFader.vue';
import ResponsiveControl from '../wrapper/ResponsiveControl.vue';
import { useAssetResult } from '../../stores/assetResult';
import { useShowState } from '../../stores/showState';
import type { Cue } from '../../types/Cue';
import { useI18n } from 'vue-i18n';
import { useShowModel } from '../../stores/showModel';
import { useApi } from '../../api';
import { useUiState } from '../../stores/uiState';
import WaveformEditor from './WaveformEditor.vue';
import { breakpointsTailwind, useBreakpoints } from '@vueuse/core';
import ButtonWrapper from '../wrapper/ButtonWrapper.vue';
import CheckboxWrapper from '../wrapper/CheckboxWrapper.vue';
import Divider from 'primevue/divider';

const { t } = useI18n();
const api = useApi();
const uiState = useUiState();
const breakpoints = useBreakpoints(breakpointsTailwind);
const xs = breakpoints.smaller('sm');
const smAndDown = breakpoints.smallerOrEqual('sm');
const mdAndDown = breakpoints.smallerOrEqual('md');

const showState = useShowState();
const assetResult = useAssetResult();
const showModel = useShowModel();

const selectedCue = defineModel<Cue | null>();
const emit = defineEmits(['update']);

const sliderChanging = ref(false);

const range = ref([
  selectedCue.value != null && selectedCue.value.params.type === 'audio'
    ? selectedCue.value.params.startTime
    : 0,
  selectedCue.value != null && selectedCue.value.params.type === 'audio'
    ? selectedCue.value.params.endTime
    : assetResult.getMetadata(selectedCue.value?.id)?.duration,
] as [number | null, number | null]);

const volume = ref(
  selectedCue.value != null && selectedCue.value.params.type === 'audio'
    ? selectedCue.value.params.volume
    : 0,
);

const panning = ref(
  selectedCue.value != null && selectedCue.value.params.type === 'audio'
    ? selectedCue.value.params.pan
    : 0,
);

const repeat = ref(
  selectedCue.value != null && selectedCue.value.params.type === 'audio'
    ? selectedCue.value.params.repeat
    : false,
);

watch(selectedCue, () => {
  if (selectedCue.value == null || selectedCue.value.params.type !== 'audio') {
    return;
  }
  range.value = [selectedCue.value.params.startTime, selectedCue.value.params.endTime] as [
    number | null,
    number | null,
  ];
  volume.value = selectedCue.value.params.volume;
  panning.value = selectedCue.value.params.pan;
  repeat.value = selectedCue.value.params.repeat;
});

const saveEditorValue = () => {
  if (selectedCue.value == null || sliderChanging.value === true) {
    return;
  }
  if (selectedCue.value.params.type !== 'audio') {
    return;
  }
  selectedCue.value.params.startTime = range.value[0];
  selectedCue.value.params.endTime = range.value[1];
  selectedCue.value.params.volume = volume.value;
  selectedCue.value.params.pan = panning.value;
  selectedCue.value.params.repeat = repeat.value;
  emit('update');
};

const changeActiveCueVolume = () => {
  if (selectedCue.value == null) return;
  const activeCue = showState.activeCues[selectedCue.value.id];
  if (activeCue != null) {
    api.sendSetVolume(activeCue.cueId, volume.value);
  }
};

const setVolumeToLUFS = () => {
  if (selectedCue.value == null) {
    return;
  }
  const integratedLufs = assetResult.get(selectedCue.value.id)?.integratedLufs;
  if (integratedLufs == null) return;
  volume.value = showModel.settings.audio.lufsTarget - integratedLufs;
  saveEditorValue();
  changeActiveCueVolume();
};

const setVolumeToMAX = () => {
  if (selectedCue.value == null) {
    return;
  }
  const peak = assetResult.get(selectedCue.value.id)?.peak;
  if (peak == null) return;
  volume.value = -peak;
  saveEditorValue();
  changeActiveCueVolume();
};
</script>

<template>
  <div class="flex flex-col p-3">
    <waveform-editor
      v-model="selectedCue"
      :height-px="125"
      :disabled="selectedCue != null && selectedCue.id in showState.activeCues"
      :volume="volume"
      @update="emit('update')"
    />
    <div class="flex flex-col sm:flex-row gap-0 sm:gap-3 items-center">
      <responsive-control
        :overlay="uiState.isRightSidebarOpen ? mdAndDown : smAndDown"
        :button-label="t('main.bottomEditor.timeLevels.changeVolume')"
      >
        <div class="flex flex-row grow gap-2">
          <volume-fader
            v-model="volume"
            class="grow"
            :label="t('main.bottomEditor.timeLevels.volume')"
            :direction="xs ? 'vertical' : 'horizontal'"
            :thumb-amount="breakpoints.xl ? 'full' : 'decreased'"
            @update="
              saveEditorValue();
              changeActiveCueVolume();
            "
          />
          <div class="grow-0 flex flex-col gap-1">
            <button-wrapper
              label="LUFS"
              class="h-6"
              @click="setVolumeToLUFS"
              v-tooltip.right="
                t('main.bottomEditor.timeLevels.lufsDescription', {
                  targetLUFS: showModel.settings.audio.lufsTarget,
                })
              "
            />
            <button-wrapper
              label="MAX"
              class="h-6"
              @click="setVolumeToMAX"
              v-tooltip.right="t('main.bottomEditor.timeLevels.peakDescription')"
            />
          </div>
        </div>
      </responsive-control>
      <divider layout="vertical" />
      <responsive-control
        :overlay="uiState.isRightSidebarOpen ? mdAndDown : smAndDown"
        :button-label="t('main.bottomEditor.timeLevels.changePan')"
      >
        <panning-fader
          v-model="panning"
          class="grow"
          :label="t('main.bottomEditor.timeLevels.pan')"
          :direction="xs ? 'vertical' : 'horizontal'"
          :disabled="selectedCue != null && selectedCue.id in showState.activeCues"
          @update="saveEditorValue()"
        />
      </responsive-control>
      <divider layout="vertical" />
      <checkbox-wrapper
        v-model="repeat"
        :label="t('main.bottomEditor.timeLevels.repeat')"
        :disabled="selectedCue != null && selectedCue.id in showState.activeCues"
        @update:model-value="saveEditorValue"
      />
    </div>
  </div>
</template>
