<script setup lang="ts">
import { $dt } from '@primeuix/themes';
import FloatLabel from 'primevue/floatlabel';
import Select from 'primevue/select';
import { useId } from 'vue';

const inputId = useId();

const props = defineProps<{
  items: {
    name: string;
    value: string;
    color?: string;
  }[];
  label?: string;
}>();
</script>

<template>
  <FloatLabel variant="on">
    <Select
      v-bind="$attrs"
      :options="props.items"
      class="w-full h-full"
      :label-id="inputId"
      option-label="name"
      option-value="value"
      :pt="{
        root: () => {
          return {
            style: 'background-color: var(--p-inputtext-background);',
          };
        },
        option: {
          style: 'padding: 0;',
        },
      }"
    >
      <template #option="innerProps">
        <div
          class="w-full h-full"
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
    <label :for="inputId">{{ props.label || '' }}</label>
  </FloatLabel>
</template>
