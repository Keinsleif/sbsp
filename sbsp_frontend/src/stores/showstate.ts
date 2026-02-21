import { defineStore } from 'pinia';
import { ref } from 'vue';
import { SyncData } from '../types/SyncData';
import { ActiveCue } from '../types/ActiveCue';
import { PlaybackStatus } from '../types/PlaybackStatus';
import { CueStatusEventParam } from '../types/CueStatusEventParam';
import { ShowState } from '../types/ShowState';

export const useShowState = defineStore('showstate', () => {
  const playbackCursor = ref<string | null>();
  const activeCues = ref<{ [id: string]: ActiveCue }>({});
  const syncedData = ref<{ [cueId in string]: { position: number; status: PlaybackStatus; lastSyncedAt: number } }>({});
  const latency = ref<number>(0);

  const handleSyncEvent = (data: SyncData) => {
    const lastSyncedAt = performance.now();

    for (const cue of data.cues) {
      latency.value = data.latency;
      const targetData = syncedData.value[cue.id];
      if (targetData != null) {
        targetData.position = cue.position;
        targetData.lastSyncedAt = lastSyncedAt;
      } else {
        console.warn('status may be broken!');
        syncedData.value[cue.id] = { position: 0.0, status: 'playing', lastSyncedAt };
      }
    }
  };

  const update = (state: ShowState) => {
    const lastSyncedAt = performance.now();

    updatePlaybackCursor(state.playbackCursor);

    const newSyncedData: { [cueId in string]: { position: number; status: PlaybackStatus; lastSyncedAt: number } } = {};
    const newActiveCues: { [id: string]: ActiveCue } = {};
    Object.entries(state.activeCues).forEach(([cueId, activeCue]) => {
      if (activeCue == null) return;
      newActiveCues[cueId] = activeCue;
      newSyncedData[cueId] = {
        position: activeCue.position,
        status: activeCue.status,
        lastSyncedAt,
      };
    });

    syncedData.value = newSyncedData;
    activeCues.value = newActiveCues;
  };

  const updatePlaybackCursor = (cursor: string | null) => {
    playbackCursor.value = cursor;
  };

  const handleCueStateEvent = (data: CueStatusEventParam) => {
    const lastSyncedAt = performance.now();
    console.debug(`${data.type.toUpperCase()}: ${data.cueId}`);
    switch (data.type) {
      case 'loaded':
        syncedData.value[data.cueId] = {
          position: data.position,
          status: 'loaded',
          lastSyncedAt,
        };
        activeCues.value[data.cueId] = {
          cueId: data.cueId,
          position: data.position,
          duration: data.duration,
          status: 'loaded',
          params: { type: 'none' },
        };
        break;
      case 'preWaitStarted':
        syncedData.value[data.cueId] = {
          position: 0.0,
          status: 'preWaiting',
          lastSyncedAt,
        };
        activeCues.value[data.cueId] = {
          cueId: data.cueId,
          position: 0.0,
          duration: data.duration,
          status: 'preWaiting',
          params: { type: 'none' },
        };
        break;
      case 'preWaitPaused':
        syncedData.value[data.cueId] = {
          position: data.position,
          status: 'preWaitPaused',
          lastSyncedAt,
        };
        break;
      case 'preWaitResumed': {
        const targetData = syncedData.value[data.cueId];
        if (targetData != null) {
          targetData.status = 'preWaiting';
          targetData.lastSyncedAt = lastSyncedAt;
        }
        break;
      }
      case 'preWaitStopped': {
        delete syncedData.value[data.cueId];
        break;
      }
      case 'preWaitCompleted':
        // start cue will automatically triggered in backend.
        break;
      case 'started':
        syncedData.value[data.cueId] = {
          position: 0.0,
          status: 'playing',
          lastSyncedAt,
        };
        activeCues.value[data.cueId] = {
          cueId: data.cueId,
          position: 0.0,
          duration: data.duration,
          status: 'playing',
          params: data.params,
        };
        break;
      case 'paused': {
        syncedData.value[data.cueId] = {
          position: data.position,
          status: 'paused',
          lastSyncedAt,
        };
        const activeCue = activeCues.value[data.cueId];
        if (activeCue != null) {
          activeCue.position = data.position;
        }
        break;
      }
      case 'resumed': {
        const targetData = syncedData.value[data.cueId];
        if (targetData != null) {
          targetData.status = 'playing';
          targetData.lastSyncedAt = lastSyncedAt;
        }
        break;
      }
      case 'stopping': {
        const targetData = syncedData.value[data.cueId];
        if (targetData != null) {
          targetData.status = 'stopping';
        }
        break;
      }
      case 'seeked': {
        const targetData = syncedData.value[data.cueId];
        if (targetData != null) {
          targetData.position = data.position;
          targetData.lastSyncedAt = lastSyncedAt;
        }
        const activeCue = activeCues.value[data.cueId];
        if (activeCue != null) {
          activeCue.position = data.position;
        }
        break;
      }
      case 'stopped':
      case 'completed':
      case 'error':
        delete syncedData.value[data.cueId];
    }
  };

  const handleRAF = () => {
    Object.entries(syncedData.value).forEach(([cueId, lastSyncCue]) => {
      let activeCue = activeCues.value[cueId];

      if (activeCue == null) {
        activeCues.value[cueId] = {
          cueId,
          status: lastSyncCue.status,
          position: lastSyncCue.position,
          duration: 0.0,
          params: { type: 'none' },
        };
        activeCue = activeCues.value[cueId]!;
      }

      activeCue.status = lastSyncCue.status;

      if (
        (['preWaiting', 'playing', 'stopping'] as PlaybackStatus[]).includes(lastSyncCue.status) &&
        activeCue.duration > 0
      ) {
        const elapsed = (performance.now() - lastSyncCue.lastSyncedAt) / 1000;
        if (activeCue.params.type == 'audio' && activeCue.params.repeating) {
          activeCue.position = (lastSyncCue.position + latency.value / 2 + elapsed) % activeCue.duration;
        } else {
          activeCue.position = Math.min(lastSyncCue.position + latency.value / 2 + elapsed, activeCue.duration);
        }
      }
    });
    Object.keys(activeCues.value).forEach((cueId) => {
      if (!(cueId in syncedData.value)) {
        delete activeCues.value[cueId];
      }
    });
  };

  return { playbackCursor, activeCues, update, handleSyncEvent, updatePlaybackCursor, handleRAF, handleCueStateEvent };
});
