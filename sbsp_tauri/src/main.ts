import { createApp } from "vue";
import App from "./App.vue";

// Vuetify
import 'vuetify/styles'
import { createVuetify } from 'vuetify'
import { aliases, mdi } from 'vuetify/iconsets/mdi-svg'
import { createPinia } from "pinia";
import { useShowState } from "./stores/showstate";
import { listen } from "@tauri-apps/api/event";
import { ShowState } from "./types/state/ShowState";
import { UiEvent } from "./types/event/UiEvent";
import { useShowModel } from "./stores/showmodel";
import { invoke } from "@tauri-apps/api/core";
import { ShowModel } from "./types/model/ShowModel";

const vuetify = createVuetify({
  icons: {
    defaultSet: 'mdi',
    aliases,
    sets: {
      mdi,
    },
  },
  theme: {
    defaultTheme: "system",
  },
});

const pinia = createPinia();

createApp(App).use(vuetify).use(pinia).mount("#app");

const showModel = useShowModel();
const showState = useShowState();

listen<ShowState>("backend-state-update", (event) => {
  showState.update(event.payload)
});

listen<UiEvent>("backend-event", (event) => {
  switch(event.payload.type) {
    case "playbackCursorMoved":
      break;
    case "showModelLoaded":
      invoke<ShowModel>("get_show_model").then((model) => {
        showModel.updateAll(model);
      });
      break;
    case "showModelSaved":
      alert("Show file saved to "+ event.payload.param.path);
      break;
    case "cueUpdated":
      showModel.updateCue(event.payload.param.cue);
      break;
    case "cueAdded":
      showModel.addCue(event.payload.param.cue, event.payload.param.atIndex);
      break;
    case "cueRemoved":
      showModel.removeCue(event.payload.param.cueId);
      break;
    case "cueMoved":
      showModel.moveCue(event.payload.param.cueId, event.payload.param.toIndex);
      break;
  }
});

invoke<ShowModel>("get_show_model").then((model) => {
  showModel.updateAll(model);
}).catch((e) => console.error(e.toString()));