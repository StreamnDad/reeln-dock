<script lang="ts">
  interface Props {
    width?: number;
    height?: number;
    cropMode?: string;
    padColor?: string;
    sourceWidth?: number;
    sourceHeight?: number;
  }

  let {
    width,
    height,
    cropMode,
    padColor = "#000000",
    sourceWidth = 1920,
    sourceHeight = 1080,
  }: Props = $props();

  // Compute the aspect ratios and visualization
  let outputAspect = $derived(
    width && height ? width / height : sourceWidth / sourceHeight,
  );

  let sourceAspect = $derived(sourceWidth / sourceHeight);

  // Container max dimensions
  const maxW = 280;
  const maxH = 200;

  // Output frame dimensions (scaled to fit container)
  let frameW = $derived(
    outputAspect >= maxW / maxH ? maxW : Math.round(maxH * outputAspect),
  );
  let frameH = $derived(
    outputAspect >= maxW / maxH ? Math.round(maxW / outputAspect) : maxH,
  );

  // Inner source area within the frame (shows how source maps to output)
  let innerStyle = $derived.by(() => {
    if (!width || !height) {
      return "width: 100%; height: 100%; background: var(--color-text-muted); opacity: 0.3;";
    }
    if (cropMode === "pad") {
      // Source is fit inside the output frame (letterboxed)
      if (sourceAspect > outputAspect) {
        // Source is wider — bars top/bottom
        const innerH = Math.round((outputAspect / sourceAspect) * 100);
        const topPad = Math.round((100 - innerH) / 2);
        return `width: 100%; height: ${innerH}%; top: ${topPad}%; left: 0; position: absolute; background: var(--color-text-muted); opacity: 0.3;`;
      } else {
        // Source is taller — bars left/right
        const innerW = Math.round((sourceAspect / outputAspect) * 100);
        const leftPad = Math.round((100 - innerW) / 2);
        return `width: ${innerW}%; height: 100%; left: ${leftPad}%; top: 0; position: absolute; background: var(--color-text-muted); opacity: 0.3;`;
      }
    }
    if (cropMode === "crop") {
      // Source is cropped to fill the output frame — show crop region
      return "width: 100%; height: 100%; background: var(--color-text-muted); opacity: 0.3;";
    }
    // Default: stretch
    return "width: 100%; height: 100%; background: var(--color-text-muted); opacity: 0.3;";
  });

  let dimensionLabel = $derived(
    width && height ? `${width} x ${height}` : "No dimensions set",
  );

  let aspectLabel = $derived.by(() => {
    if (!width || !height) return "";
    const gcd = (a: number, b: number): number => (b === 0 ? a : gcd(b, a % b));
    const d = gcd(width, height);
    return `${width / d}:${height / d}`;
  });
</script>

<div class="flex flex-col items-center gap-2">
  <!-- Frame container -->
  <div
    class="relative border border-border rounded overflow-hidden"
    style="width: {frameW}px; height: {frameH}px; background: {cropMode === 'pad' ? padColor : 'var(--color-bg)'};"
  >
    <div style={innerStyle}></div>
    {#if cropMode === "crop"}
      <!-- Crop indicator: dashed border showing source extends beyond -->
      <div class="absolute inset-0 border-2 border-dashed border-secondary/40 rounded pointer-events-none"></div>
    {/if}
  </div>

  <!-- Labels -->
  <div class="text-center">
    <p class="text-xs text-text-muted">{dimensionLabel}</p>
    {#if aspectLabel}
      <p class="text-[10px] text-text-muted">{aspectLabel}</p>
    {/if}
    {#if cropMode === "pad"}
      <p class="text-[10px] text-text-muted">Letterbox</p>
    {:else if cropMode === "crop"}
      <p class="text-[10px] text-text-muted">Crop to fill</p>
    {/if}
  </div>
</div>
