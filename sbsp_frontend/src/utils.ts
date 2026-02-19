import { MaybeRef, unref } from 'vue';
import { useAssetResult } from './stores/assetResult';
import { useShowModel } from './stores/showmodel';
import { useUiSettings } from './stores/uiSettings';
import type { Cue } from './types/Cue';
import type { CueParam } from './types/CueParam';
import type { Easing } from './types/Easing';
import { side } from './api';

export const secondsToFormat = (source_seconds: number | null): string => {
  if (source_seconds == null || isNaN(source_seconds)) {
    return '--:--.--';
  }
  const hour = Math.floor(source_seconds / 3600);
  const minute = Math.floor((source_seconds - 3600 * hour) / 60);
  const seconds = Math.floor(source_seconds - 3600 * hour - 60 * minute);
  const milliseconds = Math.floor((source_seconds - 3600 * hour - 60 * minute - seconds) * 100);

  const hh = ('00' + hour).slice(-2);
  const mm = ('00' + minute).slice(-2);
  const ss = ('00' + seconds).slice(-2);
  const ms = ('00' + milliseconds).slice(-2);
  let time = '';
  if (hour > 0) {
    time = `${hh}:${mm}:${ss}.${ms}`;
  } else {
    time = `${mm}:${ss}.${ms}`;
  }

  return time;
};

export const formatToSeconds = (source_format: string, acceptMinus: boolean = true): number => {
  let is_minus = false;
  let result = 0;
  if (source_format.startsWith('-')) {
    is_minus = true;
    source_format = source_format.slice(1);
  }
  const tokens = source_format.split(':');
  for (let i = 0; i < tokens.length; i++) {
    const num = Number(tokens[i]);
    if (isNaN(num) || num < 0) {
      break;
    }
    result += Math.pow(60, tokens.length - i - 1) * num;
  }

  return is_minus ? (acceptMinus ? -1 * result : 0) : result;
};

const secondsToHMR = (source_seconds: number): string => {
  const hour = Math.floor(source_seconds / 3600);
  const minute = Math.floor((source_seconds - 3600 * hour) / 60);
  const seconds = Math.floor(source_seconds - 3600 * hour - 60 * minute);
  const milliseconds = Math.floor((source_seconds - 3600 * hour - 60 * minute - seconds) * 100);

  const hh = ('00' + hour).slice(-2);
  const mm = ('00' + minute).slice(-2);
  const ss = ('00' + seconds).slice(-2);
  let ms = ('00' + milliseconds).slice(-2);

  if (ms.endsWith('0')) {
    ms = ms.slice(0, 1);
  }

  let time = '';
  if (hour > 0) {
    time = `${hh}h ${mm}m ${ss}s`;
  } else if (minute > 0) {
    time = `${mm}m ${ss}.${ms}s`;
  } else if (milliseconds == 0) {
    time = `${seconds}s`;
  } else {
    time = `${seconds}.${ms}s`;
  }

  return time;
};

export const format = (str: string, obj: { [key: string]: string }): string => {
  return str.replace(/\{\{|\}\}|\{((\w|\s)+)\}/g, (match, key) => {
    if (match === '{{') return '{';
    if (match === '}}') return '}';

    const replaceTarget = obj[key.trim()];
    return typeof replaceTarget !== 'undefined' ? replaceTarget : match;
  });
};

export const buildCueName = (cue: Cue | null): string => {
  if (cue == null) {
    return '';
  }
  const uiSettings = useUiSettings();
  const nameFormat = uiSettings.settings.nameFormat;
  switch (cue.params.type) {
    case 'audio':
      return format(nameFormat.audio, {
        filename: cue.params.target.replace(/^.*[\\/]/, ''),
      });
    case 'wait':
      return format(nameFormat.wait, {
        duration: secondsToHMR(cue.params.duration),
      });
    case 'fade': {
      const showModel = useShowModel();
      const targetCue = showModel.getCueById(cue.params.target);
      return format(nameFormat.fade, {
        targetName: buildCueName(targetCue ?? null),
      });
    }
    case 'start': {
      const showModel = useShowModel();
      const targetCue = showModel.getCueById(cue.params.target);
      return format(nameFormat.start, {
        targetName: buildCueName(targetCue ?? null),
      });
    }
    case 'stop': {
      const showModel = useShowModel();
      const targetCue = showModel.getCueById(cue.params.target);
      return format(nameFormat.stop, {
        targetName: buildCueName(targetCue ?? null),
      });
    }
    case 'pause': {
      const showModel = useShowModel();
      const targetCue = showModel.getCueById(cue.params.target);
      return format(nameFormat.pause, {
        targetName: buildCueName(targetCue ?? null),
      });
    }
    case 'load': {
      const showModel = useShowModel();
      const targetCue = showModel.getCueById(cue.params.target);
      return format(nameFormat.load, {
        targetName: buildCueName(targetCue ?? null),
      });
    }
    case 'group': {
      return nameFormat.group;
    }
    default:
      return '';
  }
};

export const calculateDuration = (cueParam: CueParam, totalDuration: number | null | undefined): number | null => {
  switch (cueParam.type) {
    case 'audio': {
      if (totalDuration == null || isNaN(totalDuration)) {
        return null;
      }
      let duration = totalDuration;
      if (cueParam.endTime != null && cueParam.endTime < totalDuration) {
        duration = cueParam.endTime;
      }
      if (cueParam.startTime != null) {
        duration -= cueParam.startTime;
      }
      return duration;
    }
    case 'wait':
      return cueParam.duration;
    case 'fade':
      return cueParam.fadeParam.duration;
    case 'start':
    case 'stop':
    case 'pause':
    case 'load':
    case 'group':
      return null;
    default:
      return null;
  }
};

export type Curve = {
  type: 'inPow' | 'outPow' | 'inOutPow' | 'linear' | null;
  power: number | null;
};

export const easingToCurve = (easing: Easing): Curve => {
  switch (easing.type) {
    case 'linear':
      return { type: 'linear', power: null };
    case 'inPowi':
    case 'inPowf':
      return { type: 'inPow', power: easing.intensity };
    case 'outPowi':
    case 'outPowf':
      return { type: 'outPow', power: easing.intensity };
    case 'inOutPowi':
    case 'inOutPowf':
      return { type: 'inOutPow', power: easing.intensity };
  }
};

export const curveToEasing = (curve: Curve): Easing => {
  if (curve.type == null) {
    curve.type = 'inOutPow';
  }
  if (curve.power == null) {
    curve.power = 2;
  }
  switch (curve.type) {
    case 'linear':
      return { type: 'linear' };
    case 'inPow':
      if (Number.isInteger(curve.power)) {
        return { type: 'inPowi', intensity: curve.power };
      } else {
        return { type: 'inPowf', intensity: curve.power };
      }
    case 'outPow':
      if (Number.isInteger(curve.power)) {
        return { type: 'outPowi', intensity: curve.power };
      } else {
        return { type: 'outPowf', intensity: curve.power };
      }
    case 'inOutPow':
      if (Number.isInteger(curve.power)) {
        return { type: 'inOutPowi', intensity: curve.power };
      } else {
        return { type: 'inOutPowf', intensity: curve.power };
      }
  }
};

export const getLockCursorToSelection = () => {
  if (side == 'host') {
    return useUiSettings().settings.general.lockCursorToSelection;
  } else if (side == 'remote') {
    return useShowModel().settings.remote.lockCursorToSelection;
  } else {
    return false;
  }
};

export const getDuration = (cue: Cue | null | undefined): number | null => {
  const assetResult = useAssetResult();
  if (cue == null) {
    return null;
  }
  switch (cue.params.type) {
    case 'wait':
      return cue.params.duration;
    case 'audio':
      return calculateDuration(cue.params, assetResult.get(cue.id)?.duration);
    case 'fade':
      return cue.params.fadeParam.duration;
    case 'start':
    case 'stop':
    case 'pause':
    case 'load':
    case 'group':
      return null;
  }
};

export function debounce(fn: (...args: unknown[]) => void, delay: MaybeRef<number>) {
  let timeoutId: NodeJS.Timeout;
  const wrap = function (...args: unknown[]) {
    wrap.debouncing = true;
    clearTimeout(timeoutId);
    timeoutId = setTimeout(() => {
      fn(...args);
      wrap.debouncing = false;
    }, unref(delay));
  };
  wrap.debouncing = false;
  wrap.clear = () => {
    clearTimeout(timeoutId);
  };
  wrap.immediate = fn;
  return wrap;
}
