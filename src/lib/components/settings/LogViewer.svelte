<script lang="ts">
  import { getLogEntries, getLogLevel, setLogLevel, clearLogs, type LogLevel } from "$lib/stores/log.svelte";

  let entries = $derived(getLogEntries());
  let level = $derived(getLogLevel());

  const levels: LogLevel[] = ["debug", "info", "warn", "error"];

  const levelColors: Record<LogLevel, string> = {
    debug: "text-text-muted",
    info: "text-secondary",
    warn: "text-yellow-400",
    error: "text-accent",
  };

  function formatTime(ts: number): string {
    const d = new Date(ts);
    return d.toLocaleTimeString("en-US", { hour12: false, hour: "2-digit", minute: "2-digit", second: "2-digit" })
      + "." + String(d.getMilliseconds()).padStart(3, "0");
  }

  let logContainer: HTMLDivElement | undefined = $state();
  let autoScroll = $state(true);

  $effect(() => {
    // Re-run when entries change
    entries.length;
    if (autoScroll && logContainer) {
      logContainer.scrollTop = logContainer.scrollHeight;
    }
  });
</script>

<div class="flex flex-col h-full">
  <div class="flex items-center gap-3 mb-3">
    <label class="text-sm text-text-muted">Level:</label>
    <select
      class="px-2 py-1 bg-bg border border-border rounded text-sm text-text focus:outline-none focus:border-secondary"
      value={level}
      onchange={(e) => setLogLevel(e.currentTarget.value as LogLevel)}
    >
      {#each levels as l}
        <option value={l}>{l.toUpperCase()}</option>
      {/each}
    </select>
    <label class="flex items-center gap-1 text-sm text-text-muted ml-auto">
      <input type="checkbox" bind:checked={autoScroll} class="accent-secondary" />
      Auto-scroll
    </label>
    <button
      class="px-3 py-1 text-xs bg-bg border border-border rounded hover:border-accent text-text-muted hover:text-accent transition-colors"
      onclick={clearLogs}
    >
      Clear
    </button>
    <span class="text-xs text-text-muted">{entries.length} entries</span>
  </div>

  <div
    bind:this={logContainer}
    class="flex-1 overflow-y-auto bg-bg rounded-lg border border-border p-2 font-mono text-xs leading-5"
  >
    {#if entries.length === 0}
      <p class="text-text-muted text-center py-4">No log entries</p>
    {:else}
      {#each entries as entry (entry.timestamp + entry.message)}
        <div class="flex gap-2 hover:bg-surface-hover px-1 rounded">
          <span class="text-text-muted shrink-0">{formatTime(entry.timestamp)}</span>
          <span class="{levelColors[entry.level]} shrink-0 w-12 uppercase">{entry.level}</span>
          <span class="text-text-muted shrink-0">[{entry.source}]</span>
          <span class="text-text break-all">{entry.message}</span>
        </div>
      {/each}
    {/if}
  </div>
</div>
