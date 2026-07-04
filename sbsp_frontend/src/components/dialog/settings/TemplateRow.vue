<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import type { Cue } from '@/types/Cue';
import { calculateDuration, getCueIcon, secondsToFormat } from '@/utils';
import { mdiArrowDown, mdiArrowExpandDown, mdiRepeat } from '@mdi/js';
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import PathIcon from '@/components/display/PathIcon.vue';

const model = defineModel<Cue>({ required: true });
const { t } = useI18n();

const props = defineProps<{
  selected: boolean;
}>();

const duration = computed(() => {
  const params = model.value.params;
  if (params.type === 'audio') {
    return calculateDuration(params, null);
  } else if (params.type === 'wait') {
    return params.duration;
  } else if (params.type === 'fade') {
    return params.fadeParam.duration;
  } else {
    return null;
  }
});
</script>

<template>
  <tr :class="[props.selected ? $style['selected-row'] : '']">
    <td
      headers="cuelist_type"
      width="160px"
    >
      <path-icon
        :icon="getCueIcon(model.params.type)"
        class="mr-2"
      />
      {{ t(`dialog.settings.global.template.${model.params.type}`) }}
    </td>
    <td
      headers="cuelist_number"
      class="text-center"
      width="50px"
    >
      {{ model.number }}
    </td>
    <td
      headers="cuelist_name"
      width="auto"
    >
      {{ model.name != null ? model.name : t('dialog.settings.global.template.builtFromCueParam') }}
    </td>
    <td
      headers="cuelist_pre_wait"
      class="p-1 text-center"
      width="100px"
    >
      <div>
        {{ secondsToFormat(model.preWait) }}
      </div>
    </td>
    <td
      headers="cuelist_duration"
      class="p-1 text-center"
      width="100px"
    >
      <div>
        {{ secondsToFormat(duration) }}
      </div>
    </td>
    <td headers="cuelist_repeat">
      <path-icon
        v-show="
          (model.params.type == 'audio' && model.params.repeat) ||
          (model.params.type == 'group' &&
            model.params.mode.type == 'playlist' &&
            model.params.mode.repeat)
        "
        :icon="mdiRepeat"
      />
    </td>
    <td headers="cuelist_chain">
      <path-icon
        v-show="model.chain.type == 'afterComplete'"
        :icon="mdiArrowExpandDown"
      />
      <path-icon
        v-show="model.chain.type == 'afterStart'"
        :icon="mdiArrowDown"
      />
    </td>
  </tr>
</template>

<style lang="css" module>
.selected-row {
  background-color: rgb(from var(--p-primary-color) r g b / 0.3);
}
</style>
