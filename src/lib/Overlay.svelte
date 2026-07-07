<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";

  type FrameSet = { normal: string[]; mirror: string[] };
  type FrameData = Record<string, FrameSet>;
  type FrameCache = Record<string, { normal: HTMLImageElement[]; mirror: HTMLImageElement[] }>;

  const FW = 134;
  const FH = 146;

  let canvas: HTMLCanvasElement;
  let ctx: CanvasRenderingContext2D | null = null;
  let frames: FrameCache = {};
  let lastPos = { x: 0, y: 0 };
  let dragOx = 0;
  let dragOy = 0;
  let unlisten: UnlistenFn | undefined;

  function loadImg(src: string): Promise<HTMLImageElement> {
    return new Promise((resolve) => {
      const img = new Image();
      img.onload = () => resolve(img);
      img.onerror = () => resolve(new Image());
      img.src = src;
    });
  }

  async function preload(data: FrameData): Promise<FrameCache> {
    const out: FrameCache = {};
    for (const [row, set] of Object.entries(data)) {
      out[row] = {
        normal: await Promise.all(set.normal.map(loadImg)),
        mirror: await Promise.all(set.mirror.map(loadImg)),
      };
    }
    return out;
  }

  function draw(row: string, idx: number, mirror: boolean) {
    if (!ctx) return;
    const set = frames[row];
    if (!set) return;
    const arr = mirror ? set.mirror : set.normal;
    const img = arr[idx];
    if (!img || !img.complete) return;
    ctx.clearRect(0, 0, FW, FH);
    ctx.drawImage(img, 0, 0, FW, FH);
  }

  onMount(async () => {
    canvas = document.querySelector("canvas")!;
    ctx = canvas.getContext("2d");
    const data = await invoke<FrameData>("get_frames");
    frames = await preload(data);
    draw("idle", 0, false);

    unlisten = await listen<{
      row: string;
      frame_idx: number;
      mirror: boolean;
      pos_x: number;
      pos_y: number;
    }>("pet-frame", (e) => {
      lastPos = { x: e.payload.pos_x, y: e.payload.pos_y };
      draw(e.payload.row, e.payload.frame_idx, e.payload.mirror);
    });
  });

  function onPointerDown(e: PointerEvent) {
    e.preventDefault();
    dragOx = e.screenX - lastPos.x;
    dragOy = e.screenY - lastPos.y;
    invoke("pet_drag", {
      stage: "press",
      x: e.screenX,
      y: e.screenY,
      dragOx,
      dragOy,
    });
  }

  function onPointerMove(e: PointerEvent) {
    if (!(e.buttons & 1)) return;
    invoke("pet_drag", {
      stage: "drag",
      x: e.screenX,
      y: e.screenY,
      dragOx,
      dragOy,
    });
  }

  function onPointerUp(e: PointerEvent) {
    invoke("pet_drag", {
      stage: "release",
      x: e.screenX,
      y: e.screenY,
      dragOx,
      dragOy,
    });
  }
</script>

<canvas
  width={FW}
  height={FH}
  onpointerdown={onPointerDown}
  onpointermove={onPointerMove}
  onpointerup={onPointerUp}
  style="touch-action: none"
></canvas>

<style>
  :global(html, body) {
    margin: 0;
    padding: 0;
    background: transparent;
    overflow: hidden;
  }
  canvas {
    display: block;
    background: transparent;
  }
</style>
