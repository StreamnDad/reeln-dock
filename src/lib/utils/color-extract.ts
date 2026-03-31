import { convertFileSrc } from "@tauri-apps/api/core";

interface HSL {
  h: number;
  s: number;
  l: number;
}

function rgbToHsl(r: number, g: number, b: number): HSL {
  r /= 255;
  g /= 255;
  b /= 255;
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const l = (max + min) / 2;
  if (max === min) return { h: 0, s: 0, l };
  const d = max - min;
  const s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
  let h = 0;
  if (max === r) h = ((g - b) / d + (g < b ? 6 : 0)) / 6;
  else if (max === g) h = ((b - r) / d + 2) / 6;
  else h = ((r - g) / d + 4) / 6;
  return { h, s, l };
}

function hslToHex(h: number, s: number, l: number): string {
  const hue2rgb = (p: number, q: number, t: number) => {
    if (t < 0) t += 1;
    if (t > 1) t -= 1;
    if (t < 1 / 6) return p + (q - p) * 6 * t;
    if (t < 1 / 2) return q;
    if (t < 2 / 3) return p + (q - p) * (2 / 3 - t) * 6;
    return p;
  };
  let r: number, g: number, b: number;
  if (s === 0) {
    r = g = b = l;
  } else {
    const q = l < 0.5 ? l * (1 + s) : l + s - l * s;
    const p = 2 * l - q;
    r = hue2rgb(p, q, h + 1 / 3);
    g = hue2rgb(p, q, h);
    b = hue2rgb(p, q, h - 1 / 3);
  }
  const toHex = (c: number) =>
    Math.round(c * 255)
      .toString(16)
      .padStart(2, "0");
  return `#${toHex(r)}${toHex(g)}${toHex(b)}`;
}

/**
 * Extract dominant colors from an image file on disk.
 * Uses an offscreen canvas to sample pixels and bucket by hue.
 */
export async function extractDominantColors(
  filePath: string,
  count: number = 5,
): Promise<string[]> {
  const src = filePath.startsWith("/") ? convertFileSrc(filePath) : filePath;

  const img = await new Promise<HTMLImageElement>((resolve, reject) => {
    const el = new Image();
    el.crossOrigin = "anonymous";
    el.onload = () => resolve(el);
    el.onerror = () => reject(new Error("Failed to load image for color extraction"));
    el.src = src;
  });

  const size = 100;
  const canvas = document.createElement("canvas");
  canvas.width = size;
  canvas.height = size;
  const ctx = canvas.getContext("2d");
  if (!ctx) throw new Error("Canvas 2D context unavailable");

  ctx.drawImage(img, 0, 0, size, size);
  const { data } = ctx.getImageData(0, 0, size, size);

  // Bucket pixels by hue (24 buckets of 15 degrees each)
  const HUE_BUCKETS = 24;
  const buckets: { h: number; s: number; l: number; count: number }[] = Array.from(
    { length: HUE_BUCKETS },
    () => ({ h: 0, s: 0, l: 0, count: 0 }),
  );

  for (let i = 0; i < data.length; i += 4) {
    const r = data[i];
    const g = data[i + 1];
    const b = data[i + 2];
    const a = data[i + 3];
    if (a < 128) continue; // skip transparent pixels

    const hsl = rgbToHsl(r, g, b);
    // Filter out near-white and near-black
    if (hsl.l > 0.9 || hsl.l < 0.1) continue;
    // Filter out very desaturated (grays)
    if (hsl.s < 0.15) continue;

    const bucket = Math.floor(hsl.h * HUE_BUCKETS) % HUE_BUCKETS;
    buckets[bucket].h += hsl.h;
    buckets[bucket].s += hsl.s;
    buckets[bucket].l += hsl.l;
    buckets[bucket].count++;
  }

  // Sort by frequency, take top N
  const sorted = buckets
    .filter((b) => b.count > 0)
    .sort((a, b) => b.count - a.count)
    .slice(0, count);

  // Convert averaged HSL back to hex
  return sorted.map((b) =>
    hslToHex(b.h / b.count, b.s / b.count, b.l / b.count),
  );
}
