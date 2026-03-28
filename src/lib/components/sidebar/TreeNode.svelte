<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    label: string;
    open?: boolean;
    children: Snippet;
  }

  let { label, open = $bindable(true), children }: Props = $props();
</script>

<div class="mb-1">
  <button
    class="flex items-center gap-1 w-full px-2 py-1 text-xs font-semibold uppercase tracking-wider text-text-muted hover:text-text transition-colors"
    onclick={() => (open = !open)}
  >
    <span class="transition-transform" class:rotate-90={open}>&#9654;</span>
    {label}
  </button>
  {#if open}
    <div class="ml-2">
      {@render children()}
    </div>
  {/if}
</div>
