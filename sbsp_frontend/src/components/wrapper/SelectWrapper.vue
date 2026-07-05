<script setup lang="ts">
import { $dt } from '@primeuix/themes';
import FloatLabel from 'primevue/floatlabel';
import Select from 'primevue/select';

const props = defineProps<{
  items: {
    name: string;
    value: unknown;
    color?: string;
  }[];
  label?: string;
}>();

const onHide = () => {
  if (document.activeElement instanceof HTMLElement) {
    document.activeElement.blur();
  }
}
</script>

<template>
  <FloatLabel variant="on" @keydown.stop>
    <Select
      v-bind="$attrs"
      :options="props.items"
      class="h-full w-full"
      option-label="name"
      option-value="value"
      :pt="{
        root: () => {
          return {
            class: 'w-full p-inputwrapper-filled',
            style: 'background-color: var(--p-inputtext-background);',
          };
        },
        option: {
          style: 'padding: 0;',
        },
        label: (opts) => {
          return {
            style: {
              backgroundColor:
                opts.context.option != null && opts.context.option.color != 'none' // primevue is patched that option contains selected option
                  ? `rgb(from ${$dt(opts.context.option.color + '.500').variable} r g b / 0.5)`
                  : undefined,
            },
          };
        },
      }"
      @hide="onHide"
    >
      <template #value="innerProps">
        {{ props.items.find((opt) => opt.value === (innerProps.value ?? null))?.name || '&nbsp;' }}
      </template>
      <template #option="innerProps">
        <div
          class="h-full w-full"
          :style="{
            backgroundColor: innerProps.option.color
              ? 'rgb(from ' + $dt(innerProps.option.color + '.500').variable + ' r g b / 0.5)'
              : '',
            padding: 'var(--p-select-option-padding)',
          }"
        >
          {{ innerProps.option.name }}
        </div>
      </template>
    </Select>
    <label>{{ props.label || '' }}</label> <!--label cannot be attachable. Cue select is not generic input form.-->
  </FloatLabel>
</template>
