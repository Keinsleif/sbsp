<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import FloatLabel from 'primevue/floatlabel';
import InputGroup from 'primevue/inputgroup';
import InputGroupAddon from 'primevue/inputgroupaddon';
import InputText from 'primevue/inputtext';
import { watch } from 'vue';
import { ref, useId } from 'vue';
import ButtonWrapper from '../wrapper/ButtonWrapper.vue';
import { mdiChevronDown, mdiChevronUp } from '@mdi/js';

const model = defineModel<number | null>();
const props = withDefaults(
  defineProps<{
    label?: string;
    prefix?: string;
    suffix?: string;
    min?: number;
    max?: number;
    step?: number;
    precision?: number;
    acceptNull?: boolean;
    disabled?: boolean;
    showButtons?: boolean;
  }>(),
  {
    precision: 0,
    step: 1,
    showButtons: false,
    acceptNull: false,
  },
);
const emit = defineEmits(['update']);
const innerModel = ref('');

const model2text = (num: number | null | undefined) => num != null ? num.toFixed(props.precision) : ''

watch([model, () => props.precision], ([newValue]) => {
  innerModel.value = model2text(newValue);
}, {immediate: true});


const save = () => {
  const origModelString = model2text(model.value);
  if (props.disabled) {
    innerModel.value = origModelString;
    return;
  }
  if (innerModel.value.trim() === origModelString) return;
  if (innerModel.value.trim() === '') {
    if (props.acceptNull) {
      // update model by null if acceptNull == true
      // innerModel also updated by watcher
      model.value = null;
      emit('update');
      return;
    } else {
      // reset if acceptNull == false and innerModel == ''
      innerModel.value = origModelString;
      return;
    }
  }

  let parseResult = parseFloat(innerModel.value);
  if (isNaN(parseResult)) {
    // reset if innerModel cannot parse to number
    innerModel.value = origModelString;
    return;
  }

  // validate max & min
  if (props.max != null && parseResult > props.max) {
    parseResult = props.max;
  } else if (props.min != null && parseResult < props.min) {
    parseResult = props.min;
  }

  // format by precision
  const newModel = Number(parseResult.toFixed(props.precision));

  // update display value by formatted value
  innerModel.value = newModel.toFixed(props.precision);

  // update model if actually value changed
  if (newModel !== model.value) {
    model.value = newModel;
    emit('update');
  }
};

const onKeydown = (e: KeyboardEvent) => {
  if (!(e.target instanceof HTMLInputElement)) return;
  switch (e.key) {
    case 'Enter':
      e.target.blur();
      break;
    case 'Escape':
      innerModel.value = model2text(model.value); // reset
      e.target.blur();
      break;
  }
};

const increment = () => {
  const base = model.value ?? props.min ?? 0;
  if (props.max != null) {
    model.value = Math.min(base + props.step, props.max);
  } else {
    model.value = base + props.step;
  }
  emit('update');
};

const decrement = () => {
  const base = model.value ?? props.max ?? 0;
  if (props.min != null) {
    model.value = Math.max(base - props.step, props.min);
  } else {
    model.value = base - props.step;
  }
  emit('update');
};

const inputId = useId();
</script>

<template>
  <input-group>
    <input-group-addon v-if="props.prefix">
      <slot name="prefix" prefix="props.prefix">{{ props.prefix }}</slot>
    </input-group-addon>
    <float-label variant="on">
      <input-text
        v-model="innerModel"
        v-bind="$attrs"
        :input-id="inputId"
        class="w-full h-full"
        :disabled="props.disabled"
        autocomplete="off"
        :pt="{
          root: () => {
            return {
              style: 'background-color: var(--p-inputtext-background);',
            };
          },
        }"
        @blur="save"
        @keydown.stop="onKeydown"
      />
      <label :for="inputId">{{ props.label || '' }}</label>
    </float-label>
    <input-group-addon v-if="props.suffix">
      <slot name="suffix" prefix="props.suffix">{{ props.suffix }}</slot>
    </input-group-addon>
    <template v-if="props.showButtons">
      <input-group-addon>
        <button-wrapper
          :icon="mdiChevronDown"
          :disabled="props.disabled"
          variant="text"
          severity="secondary"
          @click="decrement"
        />
      </input-group-addon>
      <input-group-addon>
        <button-wrapper
          :icon="mdiChevronUp"
          :disabled="props.disabled"
          variant="text"
          severity="secondary"
          @click="increment"
        />
      </input-group-addon>
    </template>
  </input-group>
</template>
