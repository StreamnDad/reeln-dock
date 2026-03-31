<script lang="ts">
  import { isCliAvailable, isPluginInstalled } from "$lib/stores/cli.svelte";
  import type { Snippet } from "svelte";

  interface Props {
    /** What's required: "cli" for any CLI, "plugin:<name>" for a specific plugin */
    requires: string;
    /** Content to render when the requirement is met */
    children: Snippet;
    /** If true, show a message when gated instead of hiding completely */
    showMessage?: boolean;
  }

  let { requires, children, showMessage = false }: Props = $props();

  let met = $derived.by(() => {
    if (requires === "cli") return isCliAvailable();
    if (requires.startsWith("plugin:")) {
      const pluginName = requires.slice(7);
      return isPluginInstalled(pluginName);
    }
    return true;
  });

  let message = $derived.by(() => {
    if (requires === "cli") return "Requires reeln CLI";
    if (requires.startsWith("plugin:")) {
      const pluginName = requires.slice(7);
      return `Requires ${pluginName} plugin`;
    }
    return "Feature unavailable";
  });
</script>

{#if met}
  {@render children()}
{:else if showMessage}
  <div class="px-2 py-1.5 text-[11px] text-text-muted bg-bg/50 border border-border/50 rounded italic">
    {message}
  </div>
{/if}
