import { defineStore } from "pinia";

export const useUiState = defineStore("uistate", {
    state: () => ({
        selected: null as number|null,
        selectedRange: null as [number,number]|null,
        isSideBarOpen: "activeCues" as "activeCues" | "levels",
        isRightSidebarOpen: true,
        isEditorOpen: true,
    }),
    actions: {
        toggleRightSidebar() {
            this.isRightSidebarOpen = !this.isRightSidebarOpen;
        },
        toggleEditor() {
            this.isEditorOpen = !this.isEditorOpen;
        },
    }
})