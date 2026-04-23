<script lang="ts">
  import type { AuthCheckResult, PluginDetail, RegistryPlugin } from "$lib/types/plugin";
  import { togglePlugin, updatePluginSettings } from "$lib/stores/plugins.svelte";
  import { installPluginViaCli } from "$lib/ipc/plugins";
  import { refreshCliStatus } from "$lib/stores/cli.svelte";
  import { log } from "$lib/stores/log.svelte";

  interface Props {
    name: string;
    status: "enabled" | "enabled_not_installed" | "disabled" | "available";
    detail?: PluginDetail;
    registryInfo?: RegistryPlugin;
    authResults?: AuthCheckResult[];
    onAdd: () => void;
    onRemove: () => void;
    onRefreshAuth?: () => Promise<AuthCheckResult[]>;
    onCancelAuth?: () => void;
  }

  let { name, status, detail, registryInfo, authResults = [], onAdd, onRemove, onRefreshAuth, onCancelAuth }: Props = $props();

  let showSettings = $state(false);
  let editing = $state(false);
  let editValues = $state<Record<string, string>>({});
  let saving = $state(false);
  let confirmRemove = $state(false);
  let adding = $state(false);
  let installing = $state(false);
  let installError = $state<string | null>(null);
  let refreshingAuth = $state(false);

  function startEditing() {
    if (!detail) return;
    const vals: Record<string, string> = {};
    for (const [key, val] of Object.entries(detail.settings)) {
      vals[key] = typeof val === "string" ? val : JSON.stringify(val);
    }
    editValues = vals;
    editing = true;
  }

  function cancelEditing() {
    editing = false;
    editValues = {};
  }

  async function handleSave() {
    saving = true;
    const parsed: Record<string, unknown> = {};
    for (const [key, val] of Object.entries(editValues)) {
      try {
        parsed[key] = JSON.parse(val);
      } catch {
        parsed[key] = val;
      }
    }
    await updatePluginSettings(name, parsed);
    editing = false;
    saving = false;
  }

  async function handleToggle() {
    await togglePlugin(name);
  }

  async function handleAdd() {
    adding = true;
    await onAdd();
    adding = false;
  }

  async function handleInstall() {
    installing = true;
    installError = null;
    try {
      const result = await installPluginViaCli(name);
      if (result.success) {
        log.info("Plugins", `Installed ${name}`);
        await refreshCliStatus();
      } else {
        installError = result.output || "Installation failed";
        log.error("Plugins", `Failed to install ${name}: ${result.output}`);
      }
    } catch (err) {
      installError = String(err);
      log.error("Plugins", `Failed to install ${name}: ${err}`);
    } finally {
      installing = false;
    }
  }

  function handleRemoveClick() {
    if (confirmRemove) {
      onRemove();
      confirmRemove = false;
    } else {
      confirmRemove = true;
      setTimeout(() => (confirmRemove = false), 3000);
    }
  }

  function isSecretKey(key: string): boolean {
    const lower = key.toLowerCase();
    return (
      lower.includes("api_key") ||
      lower.includes("secret") ||
      lower.includes("token") ||
      lower.includes("password")
    );
  }

  function formatValue(val: unknown): string {
    if (typeof val === "string") return val;
    if (typeof val === "boolean") return val ? "true" : "false";
    if (typeof val === "number") return String(val);
    return JSON.stringify(val);
  }

  let hasAuth = $derived(authResults.length > 0);

  let hasAuthenticator = $derived(
    registryInfo?.capabilities.includes("authenticator") ?? false,
  );

  async function handleRefreshAuth() {
    if (!onRefreshAuth) return;
    refreshingAuth = true;
    try {
      await onRefreshAuth();
    } finally {
      refreshingAuth = false;
    }
  }

  function authStatusColor(s: string): string {
    switch (s) {
      case "ok":
        return "text-green-400";
      case "warn":
        return "text-amber-400";
      case "expired":
        return "text-amber-400";
      case "fail":
        return "text-accent";
      default:
        return "text-text-muted";
    }
  }

  function authStatusDot(s: string): string {
    switch (s) {
      case "ok":
        return "bg-green-400";
      case "warn":
        return "bg-amber-400";
      case "expired":
        return "bg-amber-400";
      case "fail":
        return "bg-accent";
      default:
        return "bg-zinc-500";
    }
  }

  function authStatusLabel(s: string): string {
    switch (s) {
      case "ok":
        return "authenticated";
      case "warn":
        return "warning";
      case "expired":
        return "expired";
      case "fail":
        return "failed";
      case "not_configured":
        return "not configured";
      default:
        return s;
    }
  }

  let settingsEntries = $derived(
    detail ? Object.entries(detail.settings) : [],
  );
  let hasSettings = $derived(settingsEntries.length > 0);
  let isConfigured = $derived(status !== "available");

  let borderClass = $derived(
    status === "enabled"
      ? "border-secondary/40"
      : status === "enabled_not_installed"
        ? "border-amber-500/40"
        : status === "disabled"
          ? "border-border"
          : "border-border/40 border-dashed",
  );
</script>

<div class="bg-surface rounded-lg border overflow-hidden {borderClass}">
  <div class="flex items-center gap-3 p-4">
    <!-- Plugin name + description -->
    <div class="flex-1 min-w-0">
      <div class="flex items-center gap-2">
        <h3 class="font-medium">{name}</h3>
        {#if status === "enabled_not_installed"}
          <span class="px-2 py-0.5 rounded-full bg-amber-900/60 text-amber-300 text-xs">not installed</span>
        {:else if status === "available"}
          <span class="px-2 py-0.5 rounded-full bg-bg text-text-muted text-xs">not configured</span>
        {:else if status === "disabled"}
          <span class="px-2 py-0.5 rounded-full bg-bg text-text-muted text-xs">disabled</span>
        {/if}
      </div>
      {#if registryInfo}
        <p class="text-xs text-text-muted mt-0.5 truncate" title={registryInfo.description}>
          {registryInfo.description}
        </p>
      {/if}
    </div>

    <!-- Actions -->
    <div class="flex items-center gap-3 shrink-0">
      {#if isConfigured}
        <!-- Toggle switch -->
        <button
          class="relative w-10 h-5 rounded-full transition-colors cursor-pointer"
          class:bg-secondary={status === "enabled"}
          class:bg-amber-500={status === "enabled_not_installed"}
          class:bg-border={status === "disabled"}
          onclick={handleToggle}
          title={status === "enabled" || status === "enabled_not_installed" ? "Disable" : "Enable"}
        >
          <div
            class="absolute top-0.5 w-4 h-4 rounded-full bg-white shadow transition-transform"
            class:translate-x-5={status === "enabled" || status === "enabled_not_installed"}
            class:translate-x-0.5={status === "disabled"}
          ></div>
        </button>

        {#if hasSettings}
          <button
            class="px-3 py-1 text-xs border rounded transition-colors"
            class:border-secondary={showSettings}
            class:text-secondary={showSettings}
            class:border-border={!showSettings}
            class:text-text-muted={!showSettings}
            class:hover:border-text-muted={!showSettings}
            class:hover:text-text={!showSettings}
            onclick={() => (showSettings = !showSettings)}
          >
            Settings
          </button>
        {/if}

        <!-- Remove button -->
        <button
          class="px-2 py-1 text-xs rounded transition-colors {confirmRemove
            ? 'bg-accent text-white'
            : 'text-text-muted hover:text-accent border border-transparent hover:border-accent/30'}"
          onclick={handleRemoveClick}
          title="Remove from profile"
        >
          {confirmRemove ? "Confirm?" : "Remove"}
        </button>
      {:else}
        <!-- Add button -->
        <button
          class="px-3 py-1.5 text-xs font-medium border border-secondary text-secondary hover:bg-secondary hover:text-text rounded-lg transition-colors disabled:opacity-50"
          disabled={adding}
          onclick={handleAdd}
        >
          {adding ? "Adding..." : "+ Add"}
        </button>
      {/if}
    </div>
  </div>

  <!-- Capabilities -->
  {#if registryInfo && registryInfo.capabilities.length > 0}
    <div class="px-4 pb-2 flex flex-wrap gap-1">
      {#each registryInfo.capabilities as cap}
        <span class="px-1.5 py-0.5 rounded bg-bg text-text-muted text-[10px] font-mono">
          {cap.replace("hook:", "")}
        </span>
      {/each}
    </div>
  {/if}

  <!-- Auth status -->
  {#if (hasAuth || hasAuthenticator) && status === "enabled"}
    <div class="border-t border-border/50 px-4 py-3 bg-bg/30">
      <div class="flex items-center justify-between mb-2">
        <span class="text-xs font-semibold uppercase tracking-wider text-text-muted">Authorization</span>
        {#if hasAuthenticator}
          <div class="flex items-center gap-2">
            {#if refreshingAuth}
              <button
                class="px-2.5 py-1 text-[11px] font-medium border border-accent/50 text-accent hover:border-accent rounded transition-colors"
                onclick={() => onCancelAuth?.()}
              >
                Cancel
              </button>
            {/if}
            <button
              class="px-2.5 py-1 text-[11px] font-medium border border-border text-text-muted hover:text-text hover:border-secondary rounded transition-colors disabled:opacity-50"
              disabled={refreshingAuth}
              onclick={handleRefreshAuth}
            >
              {refreshingAuth ? "Authenticating..." : "Re-authenticate"}
            </button>
          </div>
        {/if}
      </div>

      {#if hasAuth}
        <div class="space-y-2">
          {#each authResults as result (result.service)}
            <div class="flex items-start gap-2">
              <span class="mt-1.5 w-2 h-2 rounded-full shrink-0 {authStatusDot(result.status)}"></span>
              <div class="flex-1 min-w-0">
                <div class="flex items-baseline gap-2">
                  <span class="text-sm font-medium">{result.service}</span>
                  <span class="text-[11px] {authStatusColor(result.status)}">{authStatusLabel(result.status)}</span>
                </div>

                {#if result.identity}
                  <p class="text-xs text-text-muted truncate" title={result.identity}>{result.identity}</p>
                {/if}

                {#if result.expires_at}
                  <p class="text-[11px] text-text-muted">
                    Expires: {new Date(result.expires_at).toLocaleDateString()}
                  </p>
                {/if}

                {#if result.scopes && result.required_scopes}
                  {@const missing = result.required_scopes.filter((s) => !result.scopes!.includes(s))}
                  {#if missing.length > 0}
                    <p class="text-[11px] text-amber-400">
                      Missing scopes: {missing.join(", ")}
                    </p>
                  {/if}
                {/if}

                {#if result.hint}
                  <p class="text-[11px] text-text-muted italic">{result.hint}</p>
                {/if}
              </div>
            </div>
          {/each}
        </div>
      {:else}
        <p class="text-xs text-text-muted">No auth status available. Click Re-authenticate to connect.</p>
      {/if}
    </div>
  {/if}

  <!-- Install action for enabled but not installed plugins -->
  {#if status === "enabled_not_installed"}
    <div class="px-4 pb-2 flex items-center gap-3">
      <p class="text-xs text-amber-400">Enabled in config but not installed.</p>
      <button
        class="px-3 py-1 text-xs font-medium bg-amber-600 hover:bg-amber-500 text-white rounded-lg transition-colors disabled:opacity-50"
        disabled={installing}
        onclick={handleInstall}
      >
        {installing ? "Installing..." : "Install"}
      </button>
    </div>
    {#if installError}
      <div class="px-4 pb-2">
        <p class="text-xs text-accent">{installError}</p>
      </div>
    {/if}
  {/if}

  <!-- Registry meta for available plugins -->
  {#if !isConfigured && registryInfo}
    <div class="px-4 pb-2 flex items-center gap-3 text-[10px] text-text-muted">
      <span>{registryInfo.package}</span>
      <span>{registryInfo.author}</span>
      <span>{registryInfo.license}</span>
      <span>v{registryInfo.min_reeln_version}+</span>
    </div>
  {/if}

  <!-- Settings panel -->
  {#if showSettings && hasSettings && detail}
    <div class="border-t border-border p-4 bg-bg/50">
      {#if editing}
        <div class="space-y-3">
          {#each Object.entries(editValues) as [key, _] (key)}
            <div>
              <label class="block text-xs text-text-muted mb-1" for="plugin-{name}-{key}">
                {key}
              </label>
              <input
                id="plugin-{name}-{key}"
                type={isSecretKey(key) ? "password" : "text"}
                bind:value={editValues[key]}
                class="w-full px-3 py-1.5 bg-bg border border-border rounded text-sm text-text font-mono focus:outline-none focus:border-secondary"
              />
            </div>
          {/each}
          <div class="flex gap-2 pt-1">
            <button
              class="px-4 py-2 bg-primary hover:bg-primary-light text-text rounded-lg text-sm transition-colors disabled:opacity-50"
              disabled={saving}
              onclick={handleSave}
            >
              {saving ? "Saving..." : "Save"}
            </button>
            <button
              class="px-4 py-2 text-text-muted hover:text-text text-sm transition-colors"
              onclick={cancelEditing}
            >
              Cancel
            </button>
          </div>
        </div>
      {:else}
        <div class="space-y-1.5 text-sm">
          {#each settingsEntries as [key, val] (key)}
            <div class="flex items-baseline gap-2 font-mono">
              <span class="text-text-muted text-xs shrink-0">{key}</span>
              <span class="text-xs truncate" title={formatValue(val)}>
                {#if isSecretKey(key) && typeof val === "string" && val.length > 0}
                  ••••••••
                {:else}
                  {formatValue(val)}
                {/if}
              </span>
            </div>
          {/each}
        </div>
        <button
          class="mt-3 px-4 py-2 text-xs border border-border text-text-muted hover:text-text hover:border-secondary rounded-lg transition-colors"
          onclick={startEditing}
        >
          Edit Settings
        </button>
      {/if}
    </div>
  {/if}
</div>
