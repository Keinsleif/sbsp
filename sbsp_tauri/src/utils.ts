import type { Cue } from './types/Cue';
import { CueParam } from './types/CueParam';

const secondsToFormat = (source_seconds: number): string => {
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

const formatToSeconds = (source_format: string, acceptMinus: boolean = true): number => {
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

const buildCueName = (cue: Cue) => {
  if (cue.params.type == 'audio') {
    return `Play ${cue.params.target.replace(/^.*[\\/]/, '')}`;
  } else {
    return `Wait ${secondsToHMR(cue.params.duration)}`;
  }
};

const calculateDuration = (cueParam: CueParam, totalDuration: number): number => {
  if (cueParam.type != 'audio') {
    return 0;
  }
  let duration = totalDuration;
  if (cueParam.endTime != null && cueParam.endTime < totalDuration) {
    duration = cueParam.endTime;
  }
  if (cueParam.startTime != null) {
    duration -= cueParam.startTime;
  }
  return duration;
};

export { secondsToFormat, formatToSeconds, buildCueName, calculateDuration };
