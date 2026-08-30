// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

import { onMounted, onUnmounted } from 'vue';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import type { UnlistenFn } from '@tauri-apps/api/event';

export type DroppedFile = { kind: 'path'; path: string } | { kind: 'file'; file: File };

export interface UseOsFileDropOptions {
  onOver?: (x: number, y: number) => void;
  onLeave?: () => void;
  onDrop: (files: DroppedFile[], x: number, y: number) => void;
  target?: () => HTMLElement | null;
}

/**
 * Registers operating-system file-drop handling for Tauri and browser environments.
 *
 * @param options - Drag event callbacks and an optional target element resolver.
 */
export function useOsFileDrop(options: UseOsFileDropOptions) {
  let unlistenTauri: UnlistenFn | null = null;
  let disposed = false;
  let target: HTMLElement | null = null;

  const dragEnter = (e: DragEvent) => e.preventDefault();
  const dragOver = (e: DragEvent) => {
    if (!e.dataTransfer?.types.includes('Files')) return;
    e.preventDefault();
    options.onOver?.(e.clientX, e.clientY);
  };
  const dragLeave = (e: DragEvent) => {
    const related = e.relatedTarget as Node | null;
    const container = target ?? document.documentElement;
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

  onMounted(() => {
    target = options.target?.() ?? null;
    if (__IS_TAURI__) {
      getCurrentWebviewWindow()
        .onDragDropEvent((event) => {
          if (disposed) return;
          const p = event.payload;
          if (p.type === 'leave') {
            options.onLeave?.();
          } else {
            const dpr = window.devicePixelRatio || 1;
            const x = p.position.x / dpr; // window offset is already applied.
            const y = p.position.y / dpr;
            switch (p.type) {
              case 'enter':
              case 'over':
                options.onOver?.(x, y);
                break;
              case 'drop':
                if (p.paths.length > 0) {
                  options.onDrop(
                    p.paths.map((path): DroppedFile => ({ kind: 'path', path })),
                    x,
                    y,
                  );
                }
                options.onLeave?.();
                break;
            }
          }
        })
        .then((unlistenFn) => {
          if (disposed) {
            unlistenFn();
          } else {
            unlistenTauri = unlistenFn;
          }
        })
        .catch((e) => {
          console.error(e);
        });
    } else {
      if (!disposed) {
        const el: HTMLElement | Window = target ?? window;
        el.addEventListener('dragenter', dragEnter as EventListener);
        el.addEventListener('dragover', dragOver as EventListener);
        el.addEventListener('dragleave', dragLeave as EventListener);
        el.addEventListener('drop', drop as EventListener);
      }
    }
  });

  onUnmounted(() => {
    disposed = true;
    if (__IS_TAURI__) {
      if (unlistenTauri != null) {
        unlistenTauri();
        unlistenTauri = null;
      }
    } else {
      const el: HTMLElement | Window = target ?? window;
      el.removeEventListener('dragenter', dragEnter as EventListener);
      el.removeEventListener('dragover', dragOver as EventListener);
      el.removeEventListener('dragleave', dragLeave as EventListener);
      el.removeEventListener('drop', drop as EventListener);
    }
  });
}
