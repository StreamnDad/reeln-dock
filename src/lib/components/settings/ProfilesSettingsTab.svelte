<script lang="ts">
  import { getConfig, setConfig } from "$lib/stores/config.svelte";
  import { getDockSettings } from "$lib/stores/config.svelte";
  import { saveRenderProfile, deleteRenderProfile, renameRenderProfile, loadConfigFromPath } from "$lib/ipc/config";
  import { listRenderProfiles } from "$lib/ipc/render";
  import type { RenderProfile, ProfilePreset } from "$lib/types/config";
  import ProfileList from "./ProfileList.svelte";
  import ProfileEditor from "./ProfileEditor.svelte";
  import ProfilePreview from "./ProfilePreview.svelte";

  let config = $derived(getConfig());
  let dockSettings = $derived(getDockSettings());

  let profiles = $state<RenderProfile[]>([]);
  let selectedKey = $state("");
  let editingProfile = $state<Partial<RenderProfile>>({});
  let originalKey = $state("");
  let dirty = $state(false);
  let saving = $state(false);
  let message = $state("");
  let sampleClip = $state("");
  let confirmDelete = $state(false);

  // Load profiles
  $effect(() => {
    loadProfiles();
  });

  async function loadProfiles() {
    try {
      profiles = await listRenderProfiles();
    } catch {
      profiles = [];
    }
  }

  async function reloadConfig() {
    if (dockSettings.reeln_config_path) {
      const loaded = await loadConfigFromPath(dockSettings.reeln_config_path);
      setConfig(loaded.config);
    }
    await loadProfiles();
  }

  function selectProfile(key: string) {
    if (dirty && !confirm("Discard unsaved changes?")) return;
    const profile = config?.render_profiles?.[key];
    if (profile) {
      selectedKey = key;
      originalKey = key;
      editingProfile = { ...profile, name: profile.name || key };
      dirty = false;
      confirmDelete = false;
    }
  }

  function createProfile(preset?: ProfilePreset) {
    if (dirty && !confirm("Discard unsaved changes?")) return;
    const baseName = preset
      ? preset.label.toLowerCase().replace(/[^a-z0-9]/g, "-").replace(/-+/g, "-")
      : "new-profile";
    let name = baseName;
    let i = 1;
    while (config?.render_profiles?.[name]) {
      name = `${baseName}-${i++}`;
    }
    selectedKey = "";
    originalKey = "";
    editingProfile = { name, ...(preset?.profile ?? {}) };
    dirty = true;
    confirmDelete = false;
  }

  function duplicateProfile() {
    if (!selectedKey || !config?.render_profiles?.[selectedKey]) return;
    if (dirty && !confirm("Discard unsaved changes?")) return;
    const source = config.render_profiles[selectedKey];
    let name = `${selectedKey}-copy`;
    let i = 1;
    while (config?.render_profiles?.[name]) {
      name = `${selectedKey}-copy-${i++}`;
    }
    selectedKey = "";
    originalKey = "";
    editingProfile = { ...source, name };
    dirty = true;
    confirmDelete = false;
  }

  async function handleDelete() {
    if (!selectedKey) return;
    if (!confirmDelete) {
      confirmDelete = true;
      return;
    }
    try {
      await deleteRenderProfile(selectedKey);
      await reloadConfig();
      selectedKey = "";
      originalKey = "";
      editingProfile = {};
      dirty = false;
      confirmDelete = false;
      message = "Profile deleted.";
      setTimeout(() => { message = ""; }, 2000);
    } catch (e) {
      message = `Error: ${e}`;
    }
  }

  function handleProfileChange(updated: Partial<RenderProfile>) {
    editingProfile = updated;
    dirty = true;
  }

  async function saveProfile() {
    const name = editingProfile.name;
    if (!name) {
      message = "Profile name is required.";
      return;
    }

    // Validate even dimensions
    if (editingProfile.width && editingProfile.width % 2 !== 0) {
      message = "Width must be an even number.";
      return;
    }
    if (editingProfile.height && editingProfile.height % 2 !== 0) {
      message = "Height must be an even number.";
      return;
    }

    saving = true;
    message = "";

    try {
      // Build the profile object to save (exclude undefined values)
      const profileData: Record<string, unknown> = {};
      if (editingProfile.name) profileData.name = editingProfile.name;
      if (editingProfile.width !== undefined) profileData.width = editingProfile.width;
      if (editingProfile.height !== undefined) profileData.height = editingProfile.height;
      if (editingProfile.crop_mode) profileData.crop_mode = editingProfile.crop_mode;
      if (editingProfile.anchor_x !== undefined) profileData.anchor_x = editingProfile.anchor_x;
      if (editingProfile.anchor_y !== undefined) profileData.anchor_y = editingProfile.anchor_y;
      if (editingProfile.pad_color) profileData.pad_color = editingProfile.pad_color;
      if (editingProfile.scale !== undefined && editingProfile.scale !== 1.0) profileData.scale = editingProfile.scale;
      if (editingProfile.speed !== undefined && editingProfile.speed !== 1.0) profileData.speed = editingProfile.speed;
      if (editingProfile.lut) profileData.lut = editingProfile.lut;
      if (editingProfile.subtitle_template) profileData.subtitle_template = editingProfile.subtitle_template;
      if (editingProfile.codec) profileData.codec = editingProfile.codec;
      if (editingProfile.preset) profileData.preset = editingProfile.preset;
      if (editingProfile.crf !== undefined) profileData.crf = editingProfile.crf;
      if (editingProfile.audio_codec) profileData.audio_codec = editingProfile.audio_codec;
      if (editingProfile.audio_bitrate) profileData.audio_bitrate = editingProfile.audio_bitrate;
      if (editingProfile.smart) profileData.smart = editingProfile.smart;
      if (editingProfile.speed_segments && editingProfile.speed_segments.length > 0) {
        profileData.speed_segments = editingProfile.speed_segments;
      }

      // Handle rename: if the name changed and there was an original key, rename first
      if (originalKey && name !== originalKey) {
        await renameRenderProfile(originalKey, name);
      }

      await saveRenderProfile(name, profileData);
      await reloadConfig();

      selectedKey = name;
      originalKey = name;
      dirty = false;
      message = "Profile saved.";
      setTimeout(() => { message = ""; }, 2000);
    } catch (e) {
      message = `Error: ${e}`;
    }
    saving = false;
  }
</script>

<div class="flex gap-4 h-[calc(100vh-200px)]">
  <!-- Left: Profile List -->
  <div class="w-[200px] flex-shrink-0">
    <ProfileList
      {profiles}
      {selectedKey}
      onselect={selectProfile}
      oncreate={createProfile}
      onduplicate={duplicateProfile}
      ondelete={handleDelete}
    />
    {#if confirmDelete}
      <p class="text-xs text-red-400 mt-1 px-1">Click Delete again to confirm.</p>
    {/if}
  </div>

  <!-- Center: Editor -->
  <div class="flex-1 min-w-0 flex flex-col overflow-hidden">
    {#if editingProfile.name !== undefined}
      {#if message}
        <p class="text-sm text-text-muted mb-2 flex-shrink-0">{message}</p>
      {/if}

      <div class="flex-1 min-h-0 overflow-y-auto">
        <ProfileEditor
          profile={editingProfile}
          {originalKey}
          onchange={handleProfileChange}
        />
      </div>

      <!-- Save button — pinned to bottom -->
      <div class="flex gap-2 pt-3 border-t border-border flex-shrink-0">
        <button
          class="px-4 py-1.5 bg-primary hover:bg-primary-light text-text rounded-lg text-sm font-medium transition-colors disabled:opacity-50"
          onclick={saveProfile}
          disabled={saving || !dirty}
        >
          {saving ? "Saving..." : dirty ? "Save Profile" : "Saved"}
        </button>
        {#if dirty}
          <button
            class="px-3 py-1.5 text-sm text-text-muted hover:text-text transition-colors"
            onclick={() => {
              if (originalKey) {
                selectProfile(originalKey);
              } else {
                editingProfile = {};
                dirty = false;
              }
            }}
          >Discard</button>
        {/if}
      </div>
    {:else}
      <div class="flex items-center justify-center h-full text-text-muted text-sm">
        <p>Select a profile or create a new one.</p>
      </div>
    {/if}
  </div>

  <!-- Right: Preview -->
  <div class="w-[320px] flex-shrink-0 overflow-y-auto">
    {#if editingProfile.name !== undefined}
      <ProfilePreview
        profile={editingProfile}
        {sampleClip}
        onsamplechange={(clip) => { sampleClip = clip; }}
      />
    {/if}
  </div>
</div>
