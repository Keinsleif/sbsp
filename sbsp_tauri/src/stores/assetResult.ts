import { defineStore } from 'pinia';
import { useShowModel } from './showmodel';
import { invoke } from '@tauri-apps/api/core';
import { AssetData } from '../types/AssetData';
import { secondsToFormat } from '../utils';

export const useAssetResult = defineStore('assetResult', {
  state: () => ({
    duration: {} as { [cue_id: string]: string },
    waveform: {} as { [cue_id: string]: number[] },
  }),
  actions: {
    updateAssetData() {
      const showModel = useShowModel();
      for (const cue of showModel.cues) {
        if (cue.params.type == 'audio') {
          this.duration[cue.id] = '--:--.--';
          invoke<[string, AssetData]>('process_asset', { cueId: cue.id }).then((value) => {
            if (value[1].duration != null) {
              this.duration[cue.id] = secondsToFormat(value[1].duration);
              this.waveform[cue.id] = value[1].waveform;
            }
          });
        } else if (cue.params.type == 'wait') {
          this.duration[cue.id] = secondsToFormat(cue.params.duration);
        }
      }
    },
  },
});
