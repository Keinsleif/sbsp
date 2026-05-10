<template>
  <v-select
    v-model="innerPerm"
    :items="[
      { title: 'Read', value: 1},
      { title: 'Edit', value: 2},
      { title: 'Control', value: 4},
    ]"
    density="compact"
    variant="outlined"
    chips
    multiple
    autocomplete="off"
    hide-details
    @update:model-value="save"
  />
</template>

<script setup lang="ts">
import { ref, watch } from 'vue';

const perm = defineModel<number>({ required: true });

const permToInner = (perm: number): number[] => {
  const permList = [];
  for (let i = 0; i < 4; i++) {
    const value = ((perm >> i) & 1) * Math.pow(2, i);
    if (value != 0) {
      permList.push(value);
    }
  }
  return permList;
};
const innerPerm = ref<number[]>(permToInner(perm.value));

watch(perm, (newValue, oldValue) => {
  if (newValue == oldValue) return;
  innerPerm.value = permToInner(newValue);
});

const save = () => {
  document.body.focus();
  perm.value = innerPerm.value.reduce((acc, cur) => acc + cur, 0);
};
</script>
