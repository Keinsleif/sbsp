<script setup lang="ts">
// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import markdownit from 'markdown-it';
import mila from 'markdown-it-link-attributes';
import { onMounted, ref } from 'vue';
import { useApi } from '../../api';
import Dialog from 'primevue/dialog';

const api = useApi();
const isThirdPartyNoticesDialogOpen = defineModel<boolean>();
const notices = ref('');

const md = markdownit({
  html: true,
});

md.use(mila, {
  attrs: {
    target: '_blank',
    rel: 'noopener',
  },
});

onMounted(() => {
  api
    .getThirdPartyNotices()
    .then((value) => (notices.value = md.render(value)))
    .catch((e) => console.error(e));
});
</script>

<template>
  <Dialog
    v-model:visible="isThirdPartyNoticesDialogOpen"
    class="w-300 overflow-hidden"
    @contextmenu.prevent
  >
    <div class="overflow-auto p-10">
      <!-- eslint-disable-next-line vue/no-v-html -->
      <div
        :class="$style['markdown']"
        v-html="notices"
      />
    </div>
  </Dialog>
</template>

<style lang="css" module>
.markdown {
  li ul {
    margin-left: 2em;
    padding: 0;
  }
  ul {
    margin-left: 2em;
  }
  hr {
    margin-bottom: 2em;
  }
  h1,
  h2,
  h3,
  h4 {
    margin-bottom: 1em;
  }
}
</style>
