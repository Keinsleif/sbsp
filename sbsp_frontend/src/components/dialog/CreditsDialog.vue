<template>
  <v-dialog v-model="isThirdPartyNoticesDialogOpen">
    <v-sheet class="pa-10">
      <div :class="$style['markdown']" v-html="notices"></div>
    </v-sheet>
    <v-divider></v-divider>
    <v-footer class="flex-grow-0 d-flex align-center ml-0 mr-0 w-100">
      <v-btn
        class="ml-auto"
        color="primary"
        :text="t('general.close')"
        @click="isThirdPartyNoticesDialogOpen = false"
      ></v-btn>
    </v-footer>
  </v-dialog>
</template>

<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import markdownit from 'markdown-it';
import { onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();
const isThirdPartyNoticesDialogOpen = defineModel<boolean>();
const notices = ref('');

const md = markdownit({
  html: true,
});

onMounted(() => {
  invoke<string>('get_third_party_notices')
    .then((value) => (notices.value = md.render(value)))
    .catch((e) => console.error(e));
});
</script>

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
