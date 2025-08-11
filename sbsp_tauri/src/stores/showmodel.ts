import { defineStore } from "pinia";

import type { ShowModel } from "../types/ShowModel";
import { Cue } from "../types/Cue";

export const useShowModel = defineStore("showmodel", {
    state: () => ({ name: "", cues: [], settings: { general: {}, template: { audio: null, wait: null } }, }) as ShowModel,
    actions: {
        updateAll(newModel: ShowModel) {
            this.name = newModel.name;
            this.cues = newModel.cues;
            this.settings = newModel.settings;
        },
        updateCue(newCue: Cue) {
            this.cues.splice(this.cues.findIndex((cue) => cue.id = newCue.id), 1, newCue);
        },
        addCue(cue: Cue, atIndex: number) {
            this.cues.splice(atIndex, 0, cue);
        },
        removeCue(cueId: string) {
            this.cues.splice(this.cues.findIndex((cue) => cue.id==cueId), 1);
        },
        moveCue(cueId: string, toIndex: number) {
            this.cues.splice(toIndex, 0, this.cues.splice(this.cues.findIndex((cue) => cue.id == cueId), 1)[0])
        }
    }
})