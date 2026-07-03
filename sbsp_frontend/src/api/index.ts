// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import type { IBackendAdapter } from './interface';
import { useTauriApi } from './tauri';
import { useWebsocketApi } from './websocket';

export const AUDIO_EXTENSIONS = [
  'aiff',
  'aif',
  'caf',
  'mp4',
  'm4a',
  'mkv',
  'mka',
  'webm',
  'ogg',
  'oga',
  'wav',
  'aac',
  'alac',
  'flac',
  'mp3',
];

export const useApi: () => IBackendAdapter = __IS_WEBSOCKET__ ? useWebsocketApi : useTauriApi;
