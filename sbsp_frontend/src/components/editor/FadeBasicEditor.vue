<template>
  <v-sheet flat class="d-flex flex-column pa-4 ga-2">
    <cue-select
      v-model="target"
      class="flex-grow-0"
      :label="t('main.bottomEditor.fade.targetCue')"
      cue-type="audio"
      :disabled="selectedCue!.id in showState.activeCues"
      @update="saveEditorValue"
    />
    <volume-fader
      class="mt-4"
      v-model="volume"
      :label="t('main.bottomEditor.fade.targetVolume')"
      :disabled="selectedCue!.id in showState.activeCues"
      @update:model-value="saveEditorValue"
      @mousedown="sliderChanging = true"
      @mouseup="
        sliderChanging = false;
        saveEditorValue();
      "
    />
    <fade-param-input
      class="align-self-start"
      v-model="fadeParam"
      :label="t('main.bottomEditor.fade.fadeParameter')"
      condition="both"
      disable-toggle
      :disabled="selectedCue!.id in showState.activeCues"
      @update="saveEditorValue"
    ></fade-param-input>
  </v-sheet>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue';
import FadeParamInput from '../input/FadeParamInput.vue';
import { useShowState } from '../../stores/showstate';
import type { Cue } from '../../types/Cue';
import { FadeParam } from '../../types/FadeParam';
import CueSelect from '../input/CueSelect.vue';
import VolumeFader from '../input/VolumeFader.vue';
import { NIL } from 'uuid';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();
const showState = useShowState();

const selectedCue = defineModel<Cue | null>();
const emit = defineEmits(['update']);

const sliderChanging = ref(false);

const target = ref(
  selectedCue.value != null && selectedCue.value.params.type == 'fade' && selectedCue.value.params.target != NIL
    ? selectedCue.value.params.target
    : '',
);

const volume = ref(
  selectedCue.value != null && selectedCue.value.params.type == 'fade' ? selectedCue.value.params.volume : 0,
);

const fadeParam = ref(
  selectedCue.value != null && selectedCue.value.params.type == 'fade'
    ? selectedCue.value.params.fadeParam
    : ({ duration: 3.0, easing: { type: 'inOutPowi', intensity: 2 } } as FadeParam),
);

watch(selectedCue, () => {
  if (selectedCue.value == null || selectedCue.value.params.type != 'fade') {
    return;
  }

  target.value = selectedCue.value.params.target;
  volume.value = selectedCue.value.params.volume;
  fadeParam.value = selectedCue.value.params.fadeParam;
});

const saveEditorValue = () => {
  if (selectedCue.value == null || sliderChanging.value === true) {
    return;
  }
  if (selectedCue.value.params.type != 'fade') {
    return;
  }
  selectedCue.value.params.target = target.value;
  selectedCue.value.params.volume = volume.value;
  selectedCue.value.params.fadeParam = fadeParam.value;
  emit('update');
};
</script>

<style lang="css" module>
.centered-input input {
  text-align: center;
}
</style>
