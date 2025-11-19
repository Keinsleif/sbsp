<template>
  <v-sheet flat class="d-flex flex-column pa-2 ga-3">
    <v-sheet flat class="d-flex flex-row ga-2">
      <time-range
        v-model="range"
        :disabled="selectedCue!.id in showState.activeCues"
        :duration="assetResult.get(selectedCue?.id)?.duration || undefined"
        @update="saveEditorValue"
        @mousedown="sliderChanging = true"
        @mouseup="
          sliderChanging = false;
          saveEditorValue();
        "
      ></time-range>
      <v-btn-group variant="tonal" divided>
        <v-tooltip target="cursor">
          <template v-slot:activator="{ props: activatorProps }">
            <v-btn
              v-bind="activatorProps"
              :icon="mdiSkipNext"
              :disabled="selectedCue!.id in showState.activeCues"
              @click="skipFirstSilence"
            ></v-btn>
          </template>
          <span>{{ t('main.bottomEditor.timeLevels.skipFirstSilence') }}</span>
        </v-tooltip>
        <v-tooltip target="cursor">
          <template v-slot:activator="{ props: activatorProps }">
            <v-btn
              v-bind="activatorProps"
              :icon="mdiSkipPrevious"
              :disabled="selectedCue!.id in showState.activeCues"
              @click="skipLastSilence"
            ></v-btn>
          </template>
          <span>{{ t('main.bottomEditor.timeLevels.skipLastSilence') }}</span>
        </v-tooltip>
      </v-btn-group>
    </v-sheet>
    <waveform-viewer
      :target-id="selectedCue != null ? selectedCue.id : null"
      :volume="volume"
      :start-time="range[0]"
      :end-time="range[1]"
    ></waveform-viewer>
    <div class="d-flex flex-row ga-4 align-end">
      <volume-fader
        v-model="volume"
        :label="t('main.bottomEditor.timeLevels.volume')"
        :disabled="selectedCue!.id in showState.activeCues"
        :thumb-amount="width < 1600 ? (smAndDown ? 'baseOnly' : 'decreased') : 'full'"
        @update:model-value="saveEditorValue"
        @mousedown="sliderChanging = true"
        @mouseup="
          sliderChanging = false;
          saveEditorValue();
        "
      />
      <v-btn-group variant="tonal" direction="vertical" divided>
        <v-tooltip target="cursor" :text="t('main.bottomEditor.timeLevels.lufsDescription')">
          <template v-slot:activator="{ props: activatorProps }">
            <v-btn
              v-bind="activatorProps"
              density="compact"
              height="25px"
              :disabled="selectedCue!.id in showState.activeCues"
              @click="setVolumeToLUFS"
              >LUFS</v-btn
            >
          </template>
        </v-tooltip>
        <v-tooltip target="cursor" :text="t('main.bottomEditor.timeLevels.peakDescription')">
          <template v-slot:activator="{ props: activatorProps }">
            <v-btn
              v-bind="activatorProps"
              density="compact"
              height="25px"
              :disabled="selectedCue!.id in showState.activeCues"
              @click="setVolumeToMAX"
              >MAX</v-btn
            >
          </template>
        </v-tooltip>
      </v-btn-group>
      <v-divider vertical inset thickness="2" />
      <panning-fader
        :label="t('main.bottomEditor.timeLevels.pan')"
        :disabled="selectedCue!.id in showState.activeCues"
        @update:model-value="saveEditorValue"
        @mousedown="sliderChanging = true"
        @mouseup="
          sliderChanging = false;
          saveEditorValue();
        "
      />
      <v-divider vertical inset thickness="2" />
      <v-checkbox
        v-model="repeat"
        hide-details
        density="compact"
        :label="t('main.bottomEditor.timeLevels.repeat')"
        :disabled="selectedCue!.id in showState.activeCues"
        @update:model-value="saveEditorValue"
      ></v-checkbox>
    </div>
  </v-sheet>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue';
import VolumeFader from '../input/VolumeFader.vue';
import PanningFader from '../input/PanningFader.vue';
import WaveformViewer from './WaveformViewer.vue';
import TimeRange from '../input/TimeRange.vue';
import { useAssetResult } from '../../stores/assetResult';
import { useShowState } from '../../stores/showstate';
import { mdiSkipNext, mdiSkipPrevious } from '@mdi/js';
import type { Cue } from '../../types/Cue';
import { useI18n } from 'vue-i18n';
import { useDisplay } from 'vuetify';

const { t } = useI18n();
const { smAndDown, width } = useDisplay();

const showState = useShowState();
const assetResult = useAssetResult();

const selectedCue = defineModel<Cue | null>();
const emit = defineEmits(['update']);

const sliderChanging = ref(false);

const range = ref([
  selectedCue.value != null && selectedCue.value.params.type == 'audio' ? selectedCue.value.params.startTime : 0,
  selectedCue.value != null && selectedCue.value.params.type == 'audio'
    ? selectedCue.value.params.endTime
    : assetResult.get(selectedCue.value?.id)?.duration,
] as [number | null, number | null]);

const volume = ref(
  selectedCue.value != null && selectedCue.value.params.type == 'audio' ? selectedCue.value.params.volume : 0,
);

const panning = ref(
  selectedCue.value != null && selectedCue.value.params.type == 'audio' ? selectedCue.value.params.pan : 0,
);

const repeat = ref(
  selectedCue.value != null && selectedCue.value.params.type == 'audio' ? selectedCue.value.params.repeat : false,
);

watch(selectedCue, () => {
  if (selectedCue.value == null || selectedCue.value.params.type != 'audio') {
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
  if (selectedCue.value.params.type != 'audio') {
    return;
  }
  selectedCue.value.params.startTime = range.value[0];
  selectedCue.value.params.endTime = range.value[1];
  selectedCue.value.params.volume = volume.value;
  selectedCue.value.params.pan = panning.value;
  selectedCue.value.params.repeat = repeat.value;
  emit('update');
};

const skipFirstSilence = () => {
  if (selectedCue.value == null) {
    return;
  }
  const startTime = assetResult.get(selectedCue.value.id)?.startTime;
  if (startTime == null) return;
  range.value[0] = startTime;
  saveEditorValue();
};

const skipLastSilence = () => {
  if (selectedCue.value == null) {
    return;
  }
  const endTime = assetResult.get(selectedCue.value.id)?.endTime;
  if (endTime == null) return;
  range.value[1] = endTime;
  saveEditorValue();
};

const setVolumeToLUFS = () => {
  if (selectedCue.value == null) {
    return;
  }
  const integratedLufs = assetResult.get(selectedCue.value.id)?.integratedLufs;
  if (integratedLufs == null) return;
  volume.value = -14 - integratedLufs;
  saveEditorValue();
};

const setVolumeToMAX = () => {
  if (selectedCue.value == null) {
    return;
  }
  const peak = assetResult.get(selectedCue.value.id)?.peak;
  if (peak == null) return;
  volume.value = -peak;
  saveEditorValue();
};
</script>

<style lang="css" module>
.centered-input input {
  text-align: center;
}
</style>
