<script lang="ts">
  import { _ } from "svelte-i18n";
  import { User, Users, UserPlus, Trash2 } from "lucide-svelte";
  import Modal from "./Modal.svelte";
  import { autofocus } from "../lib/autofocus";

  export let flathubState: string;
  export let onAdd: (scope: "user" | "system" | "both") => void;
  export let onRemove: () => void;
  export let onClose: () => void;

  function scopeText(state: string): string {
    if (state === "both") return $_("software.scopeTextBoth");
    if (state === "user") return $_("software.scopeTextUser");
    return $_("software.scopeTextSystem");
  }

  const optionClass =
    "flex items-start gap-3 px-3.5 py-3 rounded-lg border border-border hover:border-primary hover:bg-primary/5 text-left transition-colors";
</script>

<Modal variant="confirm" title={$_("software.addFlathub")} showFooter={false} {onClose}>
  <p class="text-text-muted mb-3">{$_("software.addFlathubQuestion")}</p>
  <div class="flex flex-col gap-2">
    <button class={optionClass} use:autofocus on:click={() => onAdd("user")}>
      <User size={18} class="text-primary shrink-0 mt-0.5" />
      <span>
        <span class="block text-[0.88rem] font-semibold text-text-bright">{$_("software.scopeUserTitle")}</span>
        <span class="block text-[0.75rem] text-text-muted">{$_("software.scopeUserHint")}</span>
      </span>
    </button>
    <button class={optionClass} on:click={() => onAdd("system")}>
      <Users size={18} class="text-primary shrink-0 mt-0.5" />
      <span>
        <span class="block text-[0.88rem] font-semibold text-text-bright">{$_("software.scopeSystemTitle")}</span>
        <span class="block text-[0.75rem] text-text-muted">{$_("software.scopeSystemHint")}</span>
      </span>
    </button>
    <button class={optionClass} on:click={() => onAdd("both")}>
      <UserPlus size={18} class="text-primary shrink-0 mt-0.5" />
      <span>
        <span class="block text-[0.88rem] font-semibold text-text-bright">{$_("software.scopeBothTitle")}</span>
        <span class="block text-[0.75rem] text-text-muted">{$_("software.scopeBothHint")}</span>
      </span>
    </button>
    {#if flathubState !== "missing"}
      <button
        class="flex items-start gap-3 px-3.5 py-3 rounded-lg border border-danger/50 text-danger hover:bg-danger/10 text-left transition-colors"
        on:click={onRemove}
      >
        <Trash2 size={18} class="shrink-0 mt-0.5" />
        <span>
          <span class="block text-[0.88rem] font-semibold">{$_("software.removeFlathub")}</span>
          <span class="block text-[0.75rem] opacity-80">
            {$_("software.removeFlathubHint", { values: { scope: scopeText(flathubState) } })}
          </span>
        </span>
      </button>
    {/if}
  </div>
</Modal>
