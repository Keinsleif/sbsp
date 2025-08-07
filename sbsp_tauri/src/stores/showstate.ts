import { defineStore } from "pinia";
import { ShowState } from "../types/state/ShowState";

export const useShowState = defineStore("showstate", {
    state: () => ({ playbackCursor: null, activeCues: {} }) as ShowState,
    actions: {
        update(newState: ShowState) {
            this.playbackCursor = newState.playbackCursor;
            this.activeCues = newState.activeCues;
        }
    }
})