<script lang="ts">
  import type { RenderProfile, ProfilePreset } from "$lib/types/config";
  import { PROFILE_PRESETS } from "$lib/types/config";

  interface Props {
    profiles: RenderProfile[];
    selectedKey: string;
    onselect: (key: string) => void;
    oncreate: (preset?: ProfilePreset) => void;
    onduplicate: () => void;
    ondelete: () => void;
  }

  let { profiles, selectedKey, onselect, oncreate, onduplicate, ondelete }: Props = $props();

  let showPresets = $state(false);

  function profileDimensions(profile: RenderProfile): string {
    if (profile.width && profile.height) return `${profile.width}x${profile.height}`;
    return "";
  }
</script>

<div class="flex flex-col h-full">
  <!-- Actions -->
  <div class="space-y-1.5 mb-3">
    <button
      class="w-full px-3 py-1.5 bg-primary hover:bg-primary-light text-text rounded text-sm font-medium transition-colors"
      onclick={() => oncreate()}
    >+ New Profile</button>

    <div class="relative">
      <button
        class="w-full px-3 py-1.5 bg-bg border border-border hover:border-secondary rounded text-sm text-text-muted hover:text-text transition-colors"
        onclick={() => { showPresets = !showPresets; }}
      >From Preset...</button>

      {#if showPresets}
        <div class="absolute z-10 top-full left-0 right-0 mt-1 bg-surface border border-border rounded shadow-lg overflow-hidden">
          {#each PROFILE_PRESETS as preset}
            <button
              class="w-full px-3 py-1.5 text-left text-sm text-text hover:bg-bg transition-colors"
              onclick={() => { oncreate(preset); showPresets = false; }}
            >{preset.label}</button>
          {/each}
        </div>
      {/if}
    </div>
  </div>

  <!-- Profile list -->
  <div class="flex-1 overflow-y-auto space-y-0.5">
    {#each profiles as profile}
      {@const key = profile.name}
      <button
        class="w-full px-3 py-2 text-left rounded text-sm transition-colors"
        class:bg-primary={selectedKey === key}
        class:text-text={selectedKey === key}
        class:text-text-muted={selectedKey !== key}
        class:hover:bg-bg={selectedKey !== key}
        onclick={() => onselect(key)}
      >
        <div class="font-medium truncate">{key}</div>
        {#if profileDimensions(profile)}
          <div class="text-[10px] {selectedKey === key ? 'text-text/70' : 'text-text-muted'}">{profileDimensions(profile)}</div>
        {/if}
      </button>
    {/each}

    {#if profiles.length === 0}
      <p class="text-xs text-text-muted px-3 py-4 text-center">No profiles yet. Create one above.</p>
    {/if}
  </div>

  <!-- Bottom actions -->
  {#if selectedKey}
    <div class="flex gap-1.5 pt-3 border-t border-border mt-3">
      <button
        class="flex-1 px-2 py-1.5 text-xs bg-bg border border-border rounded text-text-muted hover:text-text hover:border-secondary transition-colors"
        onclick={onduplicate}
      >Duplicate</button>
      <button
        class="flex-1 px-2 py-1.5 text-xs bg-bg border border-border rounded text-red-400 hover:text-red-300 hover:border-red-400/50 transition-colors"
        onclick={ondelete}
      >Delete</button>
    </div>
  {/if}
</div>
