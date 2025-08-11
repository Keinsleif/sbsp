import { defineStore } from "pinia";

export const useUiState = defineStore("uistate", {
    state: () => ({
        selected: null as number|null,
        selectedRows: [] as number[],
        sideBarTab: "activeCues" as "activeCues" | "levels",
        editorTab: "basics",
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