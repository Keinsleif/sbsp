// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { onMounted, onUnmounted } from 'vue';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import type { UnlistenFn } from '@tauri-apps/api/event';

export type DroppedFile =
  | { kind: 'path'; path: string }
  | { kind: 'file'; file: File };

export interface UseOsFileDropOptions {
  onOver?: (x: number, y: number) => void;
  onLeave?: () => void;
  onDrop: (files: DroppedFile[], x: number, y: number) => void;
  target?: () => HTMLElement | null;
}

export function useOsFileDrop(options: UseOsFileDropOptions) {
  let unlistenTauri: UnlistenFn | null = null;

  const dragEnter = (e: DragEvent) => e.preventDefault();
  const dragOver = (e: DragEvent) => {
    if (!e.dataTransfer?.types.includes('Files')) return;
    e.preventDefault();
    options.onOver?.(e.clientX, e.clientY);
  };
  const dragLeave = (e: DragEvent) => {
    const related = e.relatedTarget as Node | null;
    const container = options.target?.() ?? document.body;
    if (related == null || !container.contains(related)) {
      options.onLeave?.();
    }
  };
  const drop = (e: DragEvent) => {
    e.preventDefault();
    const files = Array.from(e.dataTransfer?.files ?? []).map(
      (file): DroppedFile => ({ kind: 'file', file }),
    );
    if (files.length > 0) options.onDrop(files, e.clientX, e.clientY);
    options.onLeave?.();
  };

  onMounted(async () => {
    if (__IS_TAURI__) {
      const dpr = window.devicePixelRatio || 1;
      unlistenTauri = await getCurrentWebviewWindow().onDragDropEvent((event) => {
        const p = event.payload;
        const x = 'position' in p ? p.position.x / dpr : 0;
        const y = 'position' in p ? p.position.y / dpr : 0;
        switch (p.type) {
          case 'enter':
          case 'over':
            options.onOver?.(x, y);
            break;
          case 'drop':
            options.onDrop(
              p.paths.map((path): DroppedFile => ({ kind: 'path', path })),
              x,
              y,
            );
            options.onLeave?.();
            break;
          case 'leave':
            options.onLeave?.();
            break;
        }
      });
    } else {
      const el: HTMLElement | Window = options.target?.() ?? window;
      el.addEventListener('dragenter', dragEnter as EventListener);
      el.addEventListener('dragover', dragOver as EventListener);
      el.addEventListener('dragleave', dragLeave as EventListener);
      el.addEventListener('drop', drop as EventListener);
    }
  });

  onUnmounted(() => {
    if (unlistenTauri != null) {
      unlistenTauri();
    } else {
      const el: HTMLElement | Window = options.target?.() ?? window;
      el.removeEventListener('dragenter', dragEnter as EventListener);
      el.removeEventListener('dragover', dragOver as EventListener);
      el.removeEventListener('dragleave', dragLeave as EventListener);
      el.removeEventListener('drop', drop as EventListener);
    }
  });
}