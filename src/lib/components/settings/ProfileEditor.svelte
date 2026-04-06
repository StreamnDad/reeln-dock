<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import type { RenderProfile, SpeedSegment } from "$lib/types/config";

  interface Props {
    profile: Partial<RenderProfile>;
    originalKey: string;
    onchange: (profile: Partial<RenderProfile>) => void;
  }

  let { profile, originalKey, onchange }: Props = $props();

  let showEncoding = $state(false);

  function update(field: string, value: unknown) {
    onchange({ ...profile, [field]: value });
  }

  function updateNumber(field: string, value: string) {
    const parsed = Number(value);
    onchange({ ...profile, [field]: value === "" ? undefined : parsed });
  }

  async function pickLut() {
    const result = await open({
      title: "Select LUT file",
      filters: [{ name: "LUT", extensions: ["cube", "3dl"] }],
      directory: false,
      multiple: false,
    });
    if (result) {
      update("lut", result as string);
    }
  }

  // Quick dimension presets
  const dimensionPresets = [
    { label: "720", value: 720 },
    { label: "1080", value: 1080 },
    { label: "1920", value: 1920 },
  ];

  // ── Speed Segments ──────────────────────────────────────────────
  let segments = $derived<SpeedSegment[]>(profile.speed_segments ?? []);
  let useSegments = $derived(segments.length > 0);

  function enableSegments() {
    // Start with a default fast-slow-fast pattern
    update("speed_segments", [
      { speed: 1.0, until: 2 },
      { speed: 0.5, until: 5 },
      { speed: 1.0, until: null },
    ]);
    // Clear uniform speed when using segments
    update("speed", undefined);
  }

  function disableSegments() {
    update("speed_segments", undefined);
  }

  function updateSegment(index: number, field: "speed" | "until", value: number | null) {
    const updated = segments.map((seg, i) =>
      i === index ? { ...seg, [field]: value } : seg,
    );
    update("speed_segments", updated);
  }

  function addSegment() {
    const lastUntil = segments.length > 0
      ? (segments[segments.length - 1].until ?? 10)
      : 0;
    // Insert before the last (unbounded) segment, or add a new one
    const updated = [...segments];
    if (updated.length > 0 && updated[updated.length - 1].until === null) {
      // Insert before the final unbounded segment
      updated.splice(updated.length - 1, 0, { speed: 1.0, until: lastUntil + 3 });
    } else {
      updated.push({ speed: 1.0, until: null });
    }
    update("speed_segments", updated);
  }

  function removeSegment(index: number) {
    const updated = segments.filter((_, i) => i !== index);
    if (updated.length === 0) {
      disableSegments();
    } else {
      update("speed_segments", updated);
    }
  }

  function segmentLabel(speed: number): string {
    if (speed < 0.8) return "slow";
    if (speed > 1.2) return "fast";
    return "normal";
  }
</script>

<div class="space-y-5 pr-2">
  <!-- Identity -->
  <div>
    <label class="block text-xs text-text-muted mb-1" for="profile-name">Profile Name</label>
    <input
      id="profile-name"
      type="text"
      value={profile.name ?? ""}
      oninput={(e) => update("name", e.currentTarget.value.toLowerCase().replace(/[^a-z0-9-_]/g, "-"))}
      class="w-full px-3 py-1.5 bg-bg border border-border rounded text-sm text-text focus:outline-none focus:border-secondary"
      placeholder="e.g. tiktok-9-16"
    />
    {#if profile.name && profile.name !== originalKey && originalKey}
      <p class="text-[10px] text-text-muted mt-1">Renaming from "{originalKey}" to "{profile.name}"</p>
    {/if}
  </div>

  <!-- Dimensions & Framing -->
  <div class="bg-surface rounded-lg border border-border p-4 space-y-3">
    <h3 class="text-xs font-semibold uppercase tracking-wider text-text-muted">Dimensions & Framing</h3>
    <p class="text-[10px] text-text-muted">Used in Short render mode. Leave blank for full-frame renders.</p>

      <div class="grid grid-cols-2 gap-3">
        <div>
          <label class="block text-xs text-text-muted mb-1" for="profile-width">Width</label>
          <input
            id="profile-width"
            type="number"
            value={profile.width ?? ""}
            oninput={(e) => updateNumber("width", e.currentTarget.value)}
            class="w-full px-3 py-1.5 bg-bg border border-border rounded text-sm text-text focus:outline-none focus:border-secondary"
            min="1"
            step="2"
          />
          <div class="flex gap-1 mt-1">
            {#each dimensionPresets as preset}
              <button
                class="px-1.5 py-0.5 text-[10px] rounded transition-colors"
                class:bg-secondary={profile.width === preset.value}
                class:text-bg={profile.width === preset.value}
                class:bg-bg={profile.width !== preset.value}
                class:text-text-muted={profile.width !== preset.value}
                class:hover:text-text={profile.width !== preset.value}
                onclick={() => update("width", preset.value)}
              >{preset.label}</button>
            {/each}
          </div>
        </div>

        <div>
          <label class="block text-xs text-text-muted mb-1" for="profile-height">Height</label>
          <input
            id="profile-height"
            type="number"
            value={profile.height ?? ""}
            oninput={(e) => updateNumber("height", e.currentTarget.value)}
            class="w-full px-3 py-1.5 bg-bg border border-border rounded text-sm text-text focus:outline-none focus:border-secondary"
            min="1"
            step="2"
          />
          <div class="flex gap-1 mt-1">
            {#each dimensionPresets as preset}
              <button
                class="px-1.5 py-0.5 text-[10px] rounded transition-colors"
                class:bg-secondary={profile.height === preset.value}
                class:text-bg={profile.height === preset.value}
                class:bg-bg={profile.height !== preset.value}
                class:text-text-muted={profile.height !== preset.value}
                class:hover:text-text={profile.height !== preset.value}
                onclick={() => update("height", preset.value)}
              >{preset.label}</button>
            {/each}
          </div>
        </div>
      </div>

      <!-- Crop Mode -->
      <div>
        <label class="block text-xs text-text-muted mb-1">Crop Mode</label>
        <div class="flex gap-1">
          {#each [{ value: "pad", label: "Pad (letterbox)" }, { value: "crop", label: "Crop (fill)" }] as option}
            <button
              class="flex-1 px-2 py-1.5 rounded text-xs font-medium transition-colors text-center"
              class:bg-secondary={(profile.crop_mode ?? "") === option.value}
              class:text-bg={(profile.crop_mode ?? "") === option.value}
              class:bg-bg={(profile.crop_mode ?? "") !== option.value}
              class:text-text-muted={(profile.crop_mode ?? "") !== option.value}
              class:border={(profile.crop_mode ?? "") !== option.value}
              class:border-border={(profile.crop_mode ?? "") !== option.value}
              onclick={() => update("crop_mode", option.value)}
            >{option.label}</button>
          {/each}
        </div>
      </div>

      <!-- Anchor (crop mode only) -->
      {#if profile.crop_mode === "crop"}
        <div class="grid grid-cols-2 gap-3">
          <div>
            <label class="block text-xs text-text-muted mb-1" for="anchor-x">Anchor X: {(profile.anchor_x ?? 0.5).toFixed(2)}</label>
            <input
              id="anchor-x"
              type="range"
              min="0"
              max="1"
              step="0.05"
              value={profile.anchor_x ?? 0.5}
              oninput={(e) => update("anchor_x", Number(e.currentTarget.value))}
              class="w-full accent-secondary"
            />
          </div>
          <div>
            <label class="block text-xs text-text-muted mb-1" for="anchor-y">Anchor Y: {(profile.anchor_y ?? 0.5).toFixed(2)}</label>
            <input
              id="anchor-y"
              type="range"
              min="0"
              max="1"
              step="0.05"
              value={profile.anchor_y ?? 0.5}
              oninput={(e) => update("anchor_y", Number(e.currentTarget.value))}
              class="w-full accent-secondary"
            />
          </div>
        </div>
      {/if}

      <!-- Pad Color (pad mode only) -->
      {#if profile.crop_mode === "pad"}
        <div class="flex items-center gap-2">
          <label class="text-xs text-text-muted" for="pad-color">Pad Color</label>
          <input
            id="pad-color"
            type="color"
            value={profile.pad_color ?? "#000000"}
            oninput={(e) => update("pad_color", e.currentTarget.value)}
            class="w-8 h-8 rounded border border-border cursor-pointer"
          />
          <input
            type="text"
            value={profile.pad_color ?? "#000000"}
            oninput={(e) => update("pad_color", e.currentTarget.value)}
            class="w-24 px-2 py-1 bg-bg border border-border rounded text-xs text-text font-mono focus:outline-none focus:border-secondary"
          />
        </div>
      {/if}

      <!-- Scale -->
      <div>
        <label class="block text-xs text-text-muted mb-1" for="profile-scale">Scale: {(profile.scale ?? 1.0).toFixed(1)}</label>
        <div class="flex items-center gap-2">
          <input
            id="profile-scale"
            type="range"
            min="0.5"
            max="3.0"
            step="0.1"
            value={profile.scale ?? 1.0}
            oninput={(e) => update("scale", Number(e.currentTarget.value))}
            class="flex-1 accent-secondary"
          />
          <button class="text-[10px] text-text-muted hover:text-text" onclick={() => update("scale", undefined)}>reset</button>
        </div>
      </div>
    </div>

  <!-- Playback / Speed -->
  <div class="bg-surface rounded-lg border border-border p-4 space-y-3">
    <div class="flex items-center justify-between">
      <h3 class="text-xs font-semibold uppercase tracking-wider text-text-muted">Playback</h3>
      {#if !useSegments}
        <button
          class="text-[10px] text-text-muted hover:text-text transition-colors"
          onclick={enableSegments}
        >+ Add timing segments</button>
      {/if}
    </div>

    {#if useSegments}
      <!-- Speed Segments Editor -->
      <p class="text-[10px] text-text-muted">Define speed changes over time. Requires CLI for rendering.</p>

      <!-- Visual segment bar -->
      <div class="flex h-6 rounded overflow-hidden border border-border">
        {#each segments as seg}
          {@const width = seg.until !== null
            ? `${Math.max(seg.until / (segments.reduce((max, s) => Math.max(max, s.until ?? max), 10)) * 100, 15)}%`
            : "1fr"}
          <div
            class="flex items-center justify-center text-[9px] font-medium border-r border-border last:border-r-0"
            style="flex: {seg.until !== null ? `0 0 ${width}` : '1 1 0'}; background: {seg.speed < 0.8 ? 'var(--color-secondary)' : seg.speed > 1.2 ? 'var(--color-primary)' : 'var(--color-bg)'}; color: {seg.speed < 0.8 || seg.speed > 1.2 ? 'var(--color-bg)' : 'var(--color-text-muted)'};"
          >
            {seg.speed}x
          </div>
        {/each}
      </div>

      <!-- Segment rows -->
      <div class="space-y-2">
        {#each segments as seg, i}
          <div class="flex items-center gap-2 px-2 py-1.5 bg-bg rounded border border-border">
            <span class="text-[10px] text-text-muted w-8 text-center font-medium">
              {segmentLabel(seg.speed)}
            </span>
            <div class="flex-1">
              <label class="block text-[10px] text-text-muted">Speed: {seg.speed.toFixed(1)}x</label>
              <input
                type="range"
                min="0.25"
                max="2.0"
                step="0.05"
                value={seg.speed}
                oninput={(e) => updateSegment(i, "speed", Number(e.currentTarget.value))}
                class="w-full accent-secondary"
              />
            </div>
            <div class="w-20">
              {#if seg.until !== null}
                <label class="block text-[10px] text-text-muted">Until {seg.until}s</label>
                <input
                  type="number"
                  min="0.5"
                  step="0.5"
                  value={seg.until}
                  oninput={(e) => updateSegment(i, "until", Number(e.currentTarget.value) || null)}
                  class="w-full px-1.5 py-0.5 bg-surface border border-border rounded text-[10px] text-text focus:outline-none focus:border-secondary"
                />
              {:else}
                <span class="block text-[10px] text-text-muted">to end</span>
              {/if}
            </div>
            <button
              class="text-text-muted hover:text-red-400 text-sm transition-colors"
              onclick={() => removeSegment(i)}
              title="Remove segment"
            >&times;</button>
          </div>
        {/each}
      </div>

      <div class="flex gap-2">
        <button
          class="px-2 py-1 text-[10px] bg-bg border border-border rounded text-text-muted hover:text-text hover:border-secondary transition-colors"
          onclick={addSegment}
        >+ Add Segment</button>
        <button
          class="px-2 py-1 text-[10px] text-text-muted hover:text-text transition-colors"
          onclick={disableSegments}
        >Use uniform speed</button>
      </div>
    {:else}
      <!-- Simple uniform speed slider -->
      <div>
        <label class="block text-xs text-text-muted mb-1" for="profile-speed">Speed: {(profile.speed ?? 1.0).toFixed(1)}x</label>
        <div class="flex items-center gap-2">
          <input
            id="profile-speed"
            type="range"
            min="0.25"
            max="2.0"
            step="0.05"
            value={profile.speed ?? 1.0}
            oninput={(e) => update("speed", Number(e.currentTarget.value))}
            class="flex-1 accent-secondary"
          />
          <button class="text-[10px] text-text-muted hover:text-text" onclick={() => update("speed", undefined)}>reset</button>
        </div>
      </div>
    {/if}
  </div>

  <!-- Color -->
  <div class="bg-surface rounded-lg border border-border p-4 space-y-3">
    <h3 class="text-xs font-semibold uppercase tracking-wider text-text-muted">Color</h3>

    <div>
      <label class="block text-xs text-text-muted mb-1" for="profile-lut">LUT File</label>
      <div class="flex items-center gap-2">
        <input
          id="profile-lut"
          type="text"
          value={profile.lut ?? ""}
          readonly
          class="flex-1 px-3 py-1.5 bg-bg border border-border rounded text-sm text-text truncate"
          placeholder="No LUT selected"
        />
        <button
          class="px-3 py-1.5 bg-bg border border-border rounded text-sm text-text-muted hover:text-text hover:border-secondary transition-colors"
          onclick={pickLut}
        >Browse</button>
        {#if profile.lut}
          <button
            class="text-[10px] text-text-muted hover:text-text"
            onclick={() => update("lut", undefined)}
          >clear</button>
        {/if}
      </div>
    </div>
  </div>

  <!-- Encoding (collapsed) -->
  <div class="bg-surface rounded-lg border border-border overflow-hidden">
    <button
      class="w-full px-4 py-3 flex items-center justify-between text-xs font-semibold uppercase tracking-wider text-text-muted hover:text-text transition-colors"
      onclick={() => { showEncoding = !showEncoding; }}
    >
      <span>Encoding</span>
      <span class="text-sm">{showEncoding ? "\u25BC" : "\u25B6"}</span>
    </button>

    {#if showEncoding}
      <div class="px-4 pb-4 space-y-3">
        <div class="grid grid-cols-2 gap-3">
          <div>
            <label class="block text-xs text-text-muted mb-1" for="profile-codec">Codec</label>
            <select
              id="profile-codec"
              value={profile.codec ?? ""}
              onchange={(e) => update("codec", e.currentTarget.value || undefined)}
              class="w-full px-3 py-1.5 bg-bg border border-border rounded text-sm text-text focus:outline-none focus:border-secondary"
            >
              <option value="">Default</option>
              <option value="libx264">H.264</option>
              <option value="libx265">H.265</option>
            </select>
          </div>
          <div>
            <label class="block text-xs text-text-muted mb-1" for="profile-preset">Preset</label>
            <select
              id="profile-preset"
              value={profile.preset ?? ""}
              onchange={(e) => update("preset", e.currentTarget.value || undefined)}
              class="w-full px-3 py-1.5 bg-bg border border-border rounded text-sm text-text focus:outline-none focus:border-secondary"
            >
              <option value="">Default</option>
              {#each ["ultrafast", "superfast", "veryfast", "faster", "fast", "medium", "slow", "slower", "veryslow"] as p}
                <option value={p}>{p}</option>
              {/each}
            </select>
          </div>
        </div>

        <div>
          <label class="block text-xs text-text-muted mb-1" for="profile-crf">
            CRF: {profile.crf ?? "default"}
            {#if profile.crf !== undefined}
              <span class="text-[10px] ml-1">
                ({profile.crf <= 15 ? "high quality" : profile.crf <= 23 ? "good" : profile.crf <= 35 ? "moderate" : "low quality"})
              </span>
            {/if}
          </label>
          <div class="flex items-center gap-2">
            <input
              id="profile-crf"
              type="range"
              min="0"
              max="51"
              step="1"
              value={profile.crf ?? 18}
              oninput={(e) => update("crf", Number(e.currentTarget.value))}
              class="flex-1 accent-secondary"
            />
            <button class="text-[10px] text-text-muted hover:text-text" onclick={() => update("crf", undefined)}>reset</button>
          </div>
        </div>

        <div class="grid grid-cols-2 gap-3">
          <div>
            <label class="block text-xs text-text-muted mb-1" for="profile-audio-codec">Audio Codec</label>
            <select
              id="profile-audio-codec"
              value={profile.audio_codec ?? ""}
              onchange={(e) => update("audio_codec", e.currentTarget.value || undefined)}
              class="w-full px-3 py-1.5 bg-bg border border-border rounded text-sm text-text focus:outline-none focus:border-secondary"
            >
              <option value="">Default</option>
              <option value="aac">AAC</option>
              <option value="opus">Opus</option>
            </select>
          </div>
          <div>
            <label class="block text-xs text-text-muted mb-1" for="profile-audio-bitrate">Audio Bitrate</label>
            <select
              id="profile-audio-bitrate"
              value={profile.audio_bitrate ?? ""}
              onchange={(e) => update("audio_bitrate", e.currentTarget.value || undefined)}
              class="w-full px-3 py-1.5 bg-bg border border-border rounded text-sm text-text focus:outline-none focus:border-secondary"
            >
              <option value="">Default</option>
              {#each ["96k", "128k", "192k", "256k", "320k"] as br}
                <option value={br}>{br}</option>
              {/each}
            </select>
          </div>
        </div>
      </div>
    {/if}
  </div>

  <!-- Overlay Template -->
  <div class="bg-surface rounded-lg border border-border p-4 space-y-3">
    <h3 class="text-xs font-semibold uppercase tracking-wider text-text-muted">Overlay Template</h3>
    <div>
      <label class="block text-xs text-text-muted mb-1" for="profile-subtitle">Template File Path</label>
      <input
        id="profile-subtitle"
        type="text"
        value={profile.subtitle_template ?? ""}
        oninput={(e) => update("subtitle_template", e.currentTarget.value || undefined)}
        class="w-full px-3 py-1.5 bg-bg border border-border rounded text-sm text-text focus:outline-none focus:border-secondary"
        placeholder="Path to overlay template"
      />
      <p class="text-[10px] text-text-muted mt-1">Builtin templates (e.g. builtin:branding) require the CLI.</p>
    </div>
  </div>

  <!-- Smart Zoom (disabled — plugin-only) -->
  <div class="bg-surface rounded-lg border border-border p-4">
    <label class="flex items-center gap-2 text-sm text-text-muted cursor-not-allowed opacity-50" title="Smart Zoom requires the OpenAI plugin">
      <input type="checkbox" checked={profile.smart ?? false} disabled class="accent-secondary" />
      Smart Zoom
      <span class="text-[10px]">(requires OpenAI plugin)</span>
    </label>
  </div>
</div>
