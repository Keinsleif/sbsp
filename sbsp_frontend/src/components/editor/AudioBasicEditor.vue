<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { mdiFileMusic } from '@mdi/js';
import { computed, ref, watch } from 'vue';
import FadeParamInput from '../input/FadeParamInput.vue';
import ResponsiveControl from '../wrapper/ResponsiveControl.vue';
import { useShowState } from '../../stores/showState';
import type { Cue } from '../../types/Cue';
import { useI18n } from 'vue-i18n';
import { useApi } from '../../api';
import { useUiState } from '../../stores/uiState';
import TextInput from '../input/TextInput.vue';
import { breakpointsTailwind, useBreakpoints } from '@vueuse/core';
import CheckboxWrapper from '../wrapper/CheckboxWrapper.vue';
import ButtonWrapper from '../wrapper/ButtonWrapper.vue';

const { t } = useI18n();
const uiState = useUiState();
const breakpoints = useBreakpoints(breakpointsTailwind, { strategy: 'max-width' });
const mdAndDown = breakpoints.smallerOrEqual('md');
const smAndDown = breakpoints.smallerOrEqual('sm');

const showState = useShowState();
const api = useApi();

const selectedCue = defineModel<Cue | null>();
const emit = defineEmits(['update']);

const target = ref<string>(
  selectedCue.value != null && selectedCue.value.params.type === 'audio'
    ? selectedCue.value.params.target
    : '',
);

const soundType = ref(
  selectedCue.value != null && selectedCue.value.params.type === 'audio'
    ? selectedCue.value.params.soundType === 'static'
    : false,
);

const fadeInParam = ref(
  selectedCue.value != null && selectedCue.value.params.type === 'audio'
    ? selectedCue.value.params.fadeInParam
    : null,
);

const fadeOutParam = ref(
  selectedCue.value != null && selectedCue.value.params.type === 'audio'
    ? selectedCue.value.params.fadeOutParam
    : null,
);

watch(selectedCue, () => {
  if (selectedCue.value == null || selectedCue.value.params.type !== 'audio') {
    return;
  }

  target.value = selectedCue.value.params.target;
  soundType.value = selectedCue.value.params.soundType === 'static';
  fadeInParam.value = selectedCue.value.params.fadeInParam;
  fadeOutParam.value = selectedCue.value.params.fadeOutParam;
});

const onTargetFieldKeyDown = (e: KeyboardEvent) => {
  if (
    !(e.target instanceof HTMLElement) ||
    selectedCue.value == null ||
    selectedCue.value.params.type !== 'audio'
  ) {
    return;
  }
  if (e.key === 'Enter') {
    e.target.blur();
  } else if (e.key === 'Escape') {
    target.value = selectedCue.value.params.target;
    e.target.blur();
  }
};

const saveEditorValue = () => {
  if (selectedCue.value == null) {
    return;
  }
  if (selectedCue.value.params.type !== 'audio') {
    return;
  }
  selectedCue.value.params.target = target.value;
  selectedCue.value.params.soundType = soundType.value ? 'static' : 'streaming';
  selectedCue.value.params.fadeInParam = fadeInParam.value;
  selectedCue.value.params.fadeOutParam = fadeOutParam.value;
  emit('update');
};

const pickFile = () => {
  document.body.focus();
  api.pickAudioAssets({ multiple: false }).then((value) => {
    const filepath = value[0];
    if (filepath != null) {
      target.value = filepath;
      saveEditorValue();
    }
  });
};

const isActive = computed(() => {
  return selectedCue.value != null && selectedCue.value.id in showState.activeCues;
})
</script>

<template>
  <div class="flex flex-col gap-3 p-4">
    <div class="flex flex-row gap-2">
      <text-input
        v-model="target"
        :label="t('main.bottomEditor.audio.targetFile')"
        :disabled="isActive"
        class="grow text-center"
        @blur="saveEditorValue"
        @keydown="onTargetFieldKeyDown"
      />
      <button-wrapper
        :disabled="isActive"
        :icon="mdiFileMusic"
        @click="pickFile"
      />
    </div>
    <checkbox-wrapper
      v-model="soundType"
      v-tooltip.bottom="t('general.forExpertWarning')"
      class="self-start"
      :disabled="isActive"
      :label="t('main.bottomEditor.audio.loadEntireFileOnMemory')"
      @update:model-value="saveEditorValue"
    />
    <div class="flex flex-col items-start justify-evenly gap-2 sm:flex-row">
      <responsive-control
        :overlay="uiState.isRightSidebarOpen ? mdAndDown : smAndDown"
        :button-label="t('main.bottomEditor.audio.changeFadeIn')"
      >
        <fade-param-input
          v-model="fadeInParam"
          :label="t('main.bottomEditor.audio.fadeIn')"
          condition="in"
          :disabled="isActive"
          @update="saveEditorValue"
        />
      </responsive-control>
      <responsive-control
        :overlay="uiState.isRightSidebarOpen ? mdAndDown : smAndDown"
        :button-label="t('main.bottomEditor.audio.changeFadeOut')"
      >
        <fade-param-input
          v-model="fadeOutParam"
          :label="t('main.bottomEditor.audio.fadeOut')"
          condition="out"
          :disabled="isActive"
          @update="saveEditorValue"
        />
      </responsive-control>
    </div>
  </div>
</template>
