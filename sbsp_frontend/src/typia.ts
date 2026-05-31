// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import typia from 'typia';
import type { Cue } from './types/Cue';
import type { GlobalHostSettings } from './types/GlobalHostSettings';
import type { GlobalRemoteSettings } from './types/GlobalRemoteSettings';

export const cueParser = typia.json.createIsParse<Cue[]>();
export const cueStringify = typia.json.createIsStringify<Cue[]>();

export const settingsValidator = typia.createValidate<GlobalHostSettings | GlobalRemoteSettings>();
