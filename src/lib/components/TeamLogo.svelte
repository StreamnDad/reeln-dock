<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { lookupTeam } from "$lib/stores/teams.svelte";
  import { getDockSettings } from "$lib/stores/config.svelte";

  interface Props {
    teamName: string;
    size?: "xs" | "sm" | "md" | "lg";
  }

  let { teamName, size = "sm" }: Props = $props();

  let dockSettings = $derived(getDockSettings());
  let showLogos = $derived(dockSettings.display?.show_logos ?? true);
  let team = $derived(lookupTeam(teamName));
  let logoPath = $derived(team?.logo_path ?? "");
  let primaryColor = $derived(team?.colors?.[0] ?? "#444");

  let imgSrc = $derived(
    logoPath
      ? logoPath.startsWith("/")
        ? convertFileSrc(logoPath)
        : logoPath
      : "",
  );

  let hidden = $state(false);

  // Reset hidden state when logo path changes
  let prevSrc = "";
  $effect(() => {
    if (imgSrc !== prevSrc) {
      prevSrc = imgSrc;
      hidden = false;
    }
  });

  const sizeClasses = {
    xs: "w-3.5 h-3.5",
    sm: "w-5 h-5",
    md: "w-8 h-8",
    lg: "w-12 h-12",
  };
</script>

{#if showLogos && imgSrc && !hidden}
  <img
    src={imgSrc}
    alt=""
    class="{sizeClasses[size]} rounded-sm object-contain shrink-0"
    onerror={() => { hidden = true; }}
  />
{:else if showLogos}
  <span
    class="{sizeClasses[size]} rounded-sm shrink-0"
    style="background: {primaryColor}"
  ></span>
{/if}
