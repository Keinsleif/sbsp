import { defineStore } from "pinia";

export const useUiState = defineStore("uistate", {
    state: () => ({
        selected: null as number|null,
        selectedRange: null as [number,number]|null,
        isSideBarOpen: "activeCues" as "activeCues" | "levels",
    }),
})