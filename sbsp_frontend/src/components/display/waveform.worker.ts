// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

export interface WorkerMessageInit {
  type: 'init';
  canvas: OffscreenCanvas;
  width: number;
  height: number;
  dpr: number;
}

export interface WorkerMessageUpdateData {
  type: 'updateData';
  waveform: number[];
}

export interface WorkerMessageRender {
  type: 'render';
  volume?: number;
  width?: number;
  height?: number;
}

type WorkerMessage = WorkerMessageInit | WorkerMessageUpdateData | WorkerMessageRender;

const F32_EPSILON = 1.1920929e-7; 
const CLIP_THRESHOLD = 1.0 + F32_EPSILON;

let canvas: OffscreenCanvas | null = null;
let ctx: OffscreenCanvasRenderingContext2D | null = null;

let waveformData: Float32Array | null = null;
let canvasWidth = 0;
let canvasHeight = 0;
let devicePixelRatio = 1;

self.onmessage = (e: MessageEvent<WorkerMessage>) => {
  const data = e.data;

  if (data.type === 'init') {
    canvas = data.canvas;
    ctx = canvas.getContext('2d');
    canvasWidth = data.width;
    canvasHeight = data.height;
    devicePixelRatio = data.dpr;
    return;
  }

  if (data.type === 'updateData') {
    if (!data.waveform) {
      waveformData = null;
    } else {
      waveformData = new Float32Array(data.waveform);
    }
    return;
  }

  if (data.type === 'render') {
    if (!canvas || !ctx) return;

    if (data.width && data.height) {
      canvasWidth = data.width;
      canvasHeight = data.height;
      canvas.width = canvasWidth * devicePixelRatio;
      canvas.height = canvasHeight * devicePixelRatio;
    }

    ctx.save();
    ctx.scale(devicePixelRatio, devicePixelRatio);
    ctx.clearRect(0, 0, canvasWidth, canvasHeight);

    const centerY = canvasHeight / 2;
    ctx.fillStyle = 'rgba(128, 128, 128, 0.8)';
    ctx.fillRect(0, centerY, canvasWidth, 1);

    if (waveformData && waveformData.length > 0 && canvasWidth > 0) {
      const amp = canvasHeight * 0.375;
      const samplePerPixel = waveformData.length / canvasWidth;
      const scaleY = data.volume ? Math.pow(10, data.volume / 20) : 1;

      const normalPath = new Path2D();
      const clippedRedPath = new Path2D();

      for (let x = 0; x < canvasWidth; x++) {
        const start = Math.floor(x * samplePerPixel);
        const end = Math.floor((x + 1) * samplePerPixel);

        let max = waveformData[start];
        if (max != null) {
          for (let j = start; j < end; j++) {
            const val = waveformData[j];
            if (val != null && val > max) max = val;
          }

          if (max > 0) {
            const scaledRatio = max * scaleY;

            const topY = centerY - scaledRatio * amp;
            const bottomY = centerY + scaledRatio * amp;

            if (data.volume == null || scaledRatio < CLIP_THRESHOLD) {
              normalPath.moveTo(x, topY);
              normalPath.lineTo(x, bottomY);
            } else {
              clippedRedPath.moveTo(x, topY);
              clippedRedPath.lineTo(x, bottomY);
            }
          }
        }
      }

      ctx.lineWidth = 1;

      ctx.strokeStyle = 'rgb(128 128 128 / 0.8)';
      ctx.stroke(normalPath);

      ctx.strokeStyle = 'rgb(200 96 96 / 0.8)';
      ctx.stroke(clippedRedPath);
    }

    ctx.restore();
  }
};