"use client";

import { useEffect, useRef } from "react";
import {
  CELL,
  MIN,
  SPARKLE,
  SQ,
  TIERS,
} from "@/lib/sparkle-config";

function pickTierAlpha(): number {
  const roll = Math.random();
  let cursor = 0;
  for (const [alpha, weight] of TIERS) {
    cursor += weight;
    if (roll <= cursor) {
      return alpha;
    }
  }
  return MIN;
}

export function SparkleBackground() {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);

  useEffect(() => {
    if (!SPARKLE.enabled) {
      return;
    }
    const canvas = canvasRef.current;
    if (!canvas) {
      return;
    }
    const ctx = canvas.getContext("2d");
    if (!ctx) {
      return;
    }

    const isMobile = window.matchMedia("(max-width: 768px)").matches;
    const fps = isMobile ? SPARKLE.fpsMobile : SPARKLE.fpsDesktop;
    const tickMs = Math.floor(1000 / fps);

    let cols = 0;
    let rows = 0;
    let alphas: Float32Array = new Float32Array();
    let raf = 0;
    let last = 0;

    const resize = () => {
      canvas.width = Math.floor(window.innerWidth * 1.2);
      canvas.height = Math.floor(window.innerHeight * 1.2);
      cols = Math.ceil(canvas.width / CELL);
      rows = Math.ceil(canvas.height / CELL);
      alphas = new Float32Array(cols * rows);
      alphas.fill(MIN);
    };

    const draw = (ts: number) => {
      raf = requestAnimationFrame(draw);
      if (ts - last < tickMs) {
        return;
      }
      last = ts;
      ctx.clearRect(0, 0, canvas.width, canvas.height);

      const spawns = Math.max(1, Math.floor((cols * rows) / SPARKLE.spawnDivisor));
      for (let i = 0; i < spawns; i++) {
        const idx = Math.floor(Math.random() * alphas.length);
        let alpha = pickTierAlpha();
        if (Math.random() < SPARKLE.hiChance) {
          alpha += SPARKLE.boostHi;
        } else {
          alpha += SPARKLE.boostMid;
        }
        alphas[idx] = Math.min(alpha, SPARKLE.maxAlpha);
      }

      for (let i = 0; i < alphas.length; i++) {
        const a = Math.max(MIN, alphas[i] * 0.92);
        alphas[i] = a;
        const x = (i % cols) * CELL;
        const y = Math.floor(i / cols) * CELL;
        const visible = Math.min(a, SPARKLE.brightSoftCap);
        ctx.fillStyle = `rgba(1, 2, 251, ${visible.toFixed(3)})`;
        ctx.fillRect(x, y, SQ, SQ);
      }
    };

    resize();
    window.addEventListener("resize", resize);
    raf = requestAnimationFrame(draw);

    return () => {
      cancelAnimationFrame(raf);
      window.removeEventListener("resize", resize);
    };
  }, []);

  return (
    <>
      <canvas id="hack26-sparkle-canvas" ref={canvasRef} aria-hidden />
    </>
  );
}
