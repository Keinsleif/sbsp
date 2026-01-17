<template>
  <svg
    :class="$style['qr-svg']"
    v-show="text != null && qrSize != null"
    xmlns="http://www.w3.org/2000/svg"
    :viewBox="`0 0 ${qrSize} ${qrSize}`"
  >
    <path :d="qrPath"></path>
  </svg>
</template>

<script setup lang="ts">
import { encode } from 'uqr';
import { watch, ref } from 'vue';
const text = defineModel<string | null>();

const qrSize = ref<number>(0);

const qrPath = ref<string>('');

watch(
  text,
  () => {
    if (text.value == null) return '';
    const qrData = encode(text.value, {
      ecc: 'L',
      border: 2,
    });

    let path = '';

    qrData.data.forEach((row, y) => {
      row.forEach((pixel, x) => {
        if (pixel) {
          path += `M${x * 10} ${y * 10}h10v10h-10z`;
        }
      });
    });

    qrSize.value = qrData.size * 10;
    qrPath.value = path;
  },
  { immediate: true },
);
</script>

<style lang="css" module>
.qr-svg {
  background-color: white;
}
</style>
