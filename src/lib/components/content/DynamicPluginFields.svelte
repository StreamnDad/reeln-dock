<script lang="ts">
  import type { PluginUIField } from "$lib/types/plugin";

  interface Props {
    fields: PluginUIField[];
    values: Record<string, unknown>;
    onchange: (id: string, value: unknown) => void;
    pluginName?: string;
  }

  let { fields, values, onchange, pluginName }: Props = $props();

  function getValue(field: PluginUIField): unknown {
    const key = field.maps_to ?? field.id;
    return values[key] ?? field.default ?? (field.type === "boolean" ? false : undefined);
  }

  function handleChange(field: PluginUIField, value: unknown) {
    const key = field.maps_to ?? field.id;
    onchange(key, value);
  }
</script>

{#if fields.length > 0}
  <div class="space-y-3">
    {#if pluginName}
      <div class="text-[10px] font-semibold uppercase tracking-wider text-text-muted">{pluginName}</div>
    {/if}
    {#each fields as field}
      {#if field.type === "boolean"}
        <label class="flex items-center gap-2 text-sm text-text-muted cursor-pointer">
          <input
            type="checkbox"
            checked={!!getValue(field)}
            onchange={() => handleChange(field, !getValue(field))}
            class="accent-secondary"
          />
          {field.label}
        </label>
        {#if field.description}
          <p class="text-[10px] text-text-muted -mt-2 ml-5">{field.description}</p>
        {/if}

      {:else if field.type === "number"}
        <div>
          <label class="block text-xs text-text-muted mb-1" for="plugin-{field.id}">
            {field.label}{getValue(field) != null ? `: ${getValue(field)}` : ""}
          </label>
          {#if field.min != null && field.max != null}
            <div class="flex items-center gap-2">
              <input
                id="plugin-{field.id}"
                type="range"
                min={field.min}
                max={field.max}
                step={field.step ?? 1}
                value={getValue(field) ?? field.min}
                oninput={(e) => handleChange(field, Number((e.target as HTMLInputElement).value))}
                class="flex-1 accent-secondary"
              />
              <button
                class="text-[10px] text-text-muted hover:text-text"
                onclick={() => handleChange(field, field.default ?? undefined)}
              >reset</button>
            </div>
          {:else}
            <input
              id="plugin-{field.id}"
              type="number"
              min={field.min}
              max={field.max}
              step={field.step}
              value={getValue(field) ?? ""}
              oninput={(e) => {
                const v = (e.target as HTMLInputElement).value;
                handleChange(field, v ? Number(v) : undefined);
              }}
              class="w-full px-2 py-1 bg-bg border border-border rounded text-sm text-text focus:outline-none focus:border-secondary"
            />
          {/if}
          {#if field.description}
            <p class="text-[10px] text-text-muted mt-0.5">{field.description}</p>
          {/if}
        </div>

      {:else if field.type === "string"}
        <div>
          <label class="block text-xs text-text-muted mb-1" for="plugin-{field.id}">{field.label}</label>
          <input
            id="plugin-{field.id}"
            type="text"
            value={String(getValue(field) ?? "")}
            oninput={(e) => handleChange(field, (e.target as HTMLInputElement).value || undefined)}
            class="w-full px-2 py-1 bg-bg border border-border rounded text-sm text-text focus:outline-none focus:border-secondary"
          />
          {#if field.description}
            <p class="text-[10px] text-text-muted mt-0.5">{field.description}</p>
          {/if}
        </div>

      {:else if field.type === "select" && field.options}
        <div>
          <label class="block text-xs text-text-muted mb-1" for="plugin-{field.id}">{field.label}</label>
          <select
            id="plugin-{field.id}"
            value={String(getValue(field) ?? "")}
            onchange={(e) => handleChange(field, (e.target as HTMLSelectElement).value || undefined)}
            class="w-full px-2 py-1 bg-bg border border-border rounded text-sm text-text focus:outline-none focus:border-secondary"
          >
            <option value="">-</option>
            {#each field.options as opt}
              <option value={opt.value}>{opt.label}</option>
            {/each}
          </select>
          {#if field.description}
            <p class="text-[10px] text-text-muted mt-0.5">{field.description}</p>
          {/if}
        </div>
      {/if}
    {/each}
  </div>
{/if}
