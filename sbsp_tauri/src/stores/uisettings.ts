import { defineStore } from 'pinia';

export const useUiSettings = defineStore('uisettings', {
  state: () => ({
    lockCursorToSelection: true,
  }),
});
