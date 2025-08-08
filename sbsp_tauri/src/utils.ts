const secondsToFormat = (source_seconds: number): string => {
  const hour = Math.floor(source_seconds / 3600);
  const minute = Math.floor((source_seconds - 3600 * hour) / 60);
  const seconds = Math.floor((source_seconds - 3600 * hour - 60 * minute));
  const milliseconds = Math.floor((source_seconds - 3600 * hour - 60 * minute - seconds) * 100);
  
  const hh = ('0' + hour).slice(-2);
  const mm = ('0' + minute).slice(-2);
  const ss = ('0' + seconds).slice(-2);
  const ms = ('0' + milliseconds).slice(-2);
  let time = "";
    if (hour > 0) {
        time = `${hh}:${mm}:${ss}.${ms}`;
    } else {
        time = `${mm}:${ss}.${ms}`
    }

  return time
}

export {secondsToFormat}