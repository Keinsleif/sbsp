<template>
  <v-text-field
    v-bind="$attrs"
    hide-details
    persistent-placeholder
    v-model="value"
    variant="outlined"
    density="compact"
    autocomplete="off"
    :class="
      props.alignInput == 'left'
        ? $style['left-input']
        : props.alignInput == 'center'
          ? $style['center-input']
          : $style['right-input']
    "
    @keydown.stop
  >
    <template v-slot:append-inner>
      <v-fade-transition leave-absolute>
        <v-icon v-if="copied" :icon="mdiCheck"></v-icon>
        <v-icon v-else :icon="mdiContentCopy" @click="copyToClipboard(value)"></v-icon>
      </v-fade-transition>
    </template>
  </v-text-field>
</template>

<script setup lang="ts">
  import { mdiCheck, mdiContentCopy } from '@mdi/js';
  import { ref } from 'vue';

  const value = defineModel<string | null>({ default: '' });
  const props = withDefaults(
    defineProps<{
      alignInput?: 'left' | 'center' | 'right';
    }>(),
    {
      alignInput: 'center',
    },
  );

  const copied = ref(false);

  const copyToClipboard = (value: string | null) => {
    if (value) {
      copied.value = true;
      navigator.clipboard.writeText(value);
      setTimeout(() => {
        copied.value = false;
      }, 2000);
    }
  };
</script>

<style lang="css" module>
  .center-input input {
    text-align: center;
  }
  .left-input input {
    text-align: left;
  }
  .right-input input {
    text-align: right;
  }
</style>
