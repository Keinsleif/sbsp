const secondsToFormat = (source_seconds: number): string => {
  const hour = Math.floor(source_seconds / 3600);
  const minute = Math.floor((source_seconds - 3600 * hour) / 60);
  const seconds = Math.floor(source_seconds - 3600 * hour - 60 * minute);
  const milliseconds = Math.floor((source_seconds - 3600 * hour - 60 * minute - seconds) * 100);

  const hh = ('0' + hour).slice(-2);
  const mm = ('0' + minute).slice(-2);
  const ss = ('0' + seconds).slice(-2);
  const ms = ('0' + milliseconds).slice(-2);
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

export { secondsToFormat, formatToSeconds };
