import { onUnmounted } from "vue";
import { useApi } from "../api";
import { LevelMeterListener } from "../api/interface";

let listenerRegistered = false;

export const useLevelMeterListener = (listener: LevelMeterListener) => {
  if (listenerRegistered) {
    console.warn('Multiple Level Meter listener is not supported. ignoring...');
    return; // ignore more than one level meter listener
  }

  const api = useApi();
  api.listenLevelMeter(listener);

  onUnmounted(() => {
    console.debug('unregistered.');
    api.unlistenLevelMeter();
  });
};
