import { onUnmounted } from "vue";
import { useShowState } from "../stores/showstate";

type PositionCallback = (positions: { [id: string]: number }) => void;

const callbacks = new Set<PositionCallback>();
let rafId: number | null = null;
let lastFlush = 0;

const loop = (timestamp: DOMHighResTimeStamp) => {
  const showState = useShowState();
  let positions;

  // 100ms for reactive update
  if (timestamp - lastFlush > 100) {
    positions = showState.calculatePosition(true);
    lastFlush = timestamp;
  } else {
    positions = showState.calculatePosition(false);
  }

  callbacks.forEach(fn => fn(positions));

  rafId = requestAnimationFrame(loop);
};

export const usePosition = (domTickFn: PositionCallback) => {
  if (domTickFn) {
    callbacks.add(domTickFn);
  }

  if (rafId == null) {
    rafId = requestAnimationFrame(loop);
  }

  onUnmounted(() => {
    callbacks.delete(domTickFn);
    if (callbacks.size === 0 && rafId != null) {
      cancelAnimationFrame(rafId);
      rafId = null;
    }
  });
};

export const usePositionTicker = () => {
  if (rafId == null) {
    rafId = requestAnimationFrame(loop);
  }

  onUnmounted(() => {
    if (callbacks.size === 0 && rafId != null) {
      cancelAnimationFrame(rafId);
      rafId = null;
    }
  });
}