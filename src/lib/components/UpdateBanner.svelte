<script lang="ts">
  import { listen } from "@tauri-apps/api/event";
  import { open } from "@tauri-apps/plugin-shell";

  interface UpdateInfo {
    name: string;
    current: string;
    latest: string;
    release_notes: string;
    release_url: string;
    published_at: string;
  }

  interface UpdateCheckResult {
    updates: UpdateInfo[];
  }

  let updates = $state<UpdateInfo[]>([]);
  let dismissed = $state(false);
  let noUpdates = $state(false);

  $effect(() => {
    listen<UpdateCheckResult>("update:available", (event) => {
      updates = event.payload.updates;
      dismissed = false;
    });
    listen("update:none", () => {
      noUpdates = true;
      setTimeout(() => { noUpdates = false; }, 5000);
    });
  });
</script>

{#if noUpdates}
  <div class="px-4 py-2 bg-green-900/50 border-b border-green-700 text-center text-sm text-green-300">
    You're up to date!
  </div>
{/if}

{#if updates.length > 0 && !dismissed}
  <div class="px-4 py-2 bg-blue-900/50 border-b border-blue-700 flex items-center gap-3 text-sm">
    <span class="text-secondary font-medium shrink-0">Updates available</span>
    <div class="flex-1 flex flex-wrap gap-3">
      {#each updates as u}
        <button
          class="text-text hover:text-secondary transition-colors"
          onclick={() => open(u.release_url)}
        >
          {u.name} {u.current} &rarr; {u.latest}
        </button>
      {/each}
    </div>
    <button
      class="text-text-muted hover:text-text text-xs shrink-0"
      onclick={() => dismissed = true}
    >Dismiss</button>
  </div>
{/if}
