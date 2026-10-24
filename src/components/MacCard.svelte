<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { _ } from "svelte-i18n";

  // What the hardware scan offers on an Apple computer: the fan daemon, the
  // battery manager and the driver of the FaceTime HD camera. The card is
  // only drawn when the scan recognised a Mac.
  export let mac: { isMac: boolean; model: string; items: { id: string; installed: boolean; removable: boolean }[] };

  const dispatch = createEventDispatcher<{ install: string; remove: string }>();
  const names: Record<string, string> = { mbpfan: "mbpfan", tlp: "TLP", facetimehd: "FaceTime HD" };
</script>

{#if mac.isMac}
  <div class="border border-border rounded p-3">
    <div class="text-[0.7rem] font-semibold text-text-muted uppercase tracking-wide mb-1">{$_("drivers.mac.title")}</div>
    <p class="text-[0.82rem] text-text-bright mb-2">{mac.model}</p>
    {#if mac.items.length === 0}
      <p class="text-[0.78rem] text-text-muted">{$_("drivers.mac.nothing")}</p>
    {/if}
    {#each mac.items as item (item.id)}
      <div class="flex items-start justify-between gap-3 py-2 border-t border-border first:border-t-0">
        <div class="min-w-0">
          <div class="flex items-center gap-2">
            <span class="text-[0.82rem] font-semibold text-text-bright">{names[item.id] ?? item.id}</span>
            <span class="text-[0.68rem] px-2 py-0.5 rounded-full {item.installed ? 'bg-success/15 text-success' : 'bg-surface-2 text-text'}">
              {item.installed ? $_("drivers.mac.installed") : $_("drivers.mac.notInstalled")}
            </span>
          </div>
          <p class="text-[0.72rem] text-text mt-0.5">{$_(`drivers.mac.hint.${item.id}`)}</p>
        </div>
        {#if !item.installed}
          <button class="px-3 py-1.5 rounded bg-primary hover:bg-primary-hover text-text-bright text-[0.78rem] font-semibold shrink-0" on:click={() => dispatch("install", item.id)}>
            {$_("common.install")}
          </button>
        {:else if item.removable}
          <button class="px-3 py-1.5 rounded border border-danger text-danger hover:bg-danger/10 text-[0.78rem] font-semibold shrink-0" on:click={() => dispatch("remove", item.id)}>
            {$_("common.uninstall")}
          </button>
        {/if}
      </div>
    {/each}
  </div>
{/if}
