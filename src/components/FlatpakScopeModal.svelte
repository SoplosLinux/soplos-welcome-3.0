<script lang="ts">
  import { _ } from "svelte-i18n";
  import { User, Users } from "lucide-svelte";
  import Modal from "./Modal.svelte";
  import { autofocus } from "../lib/autofocus";

  export let name: string;
  export let onChoose: (scope: "user" | "system") => void;
  export let onClose: () => void;
</script>

<Modal
  variant="confirm"
  title={$_("softwareTile.flatpakScopeTitle", { values: { name } })}
  showFooter={false}
  {onClose}
>
  <p class="text-text-muted mb-3">{$_("softwareTile.flatpakScopeQuestion")}</p>
  <div class="flex flex-col gap-2">
    <button
      class="flex items-start gap-3 px-3.5 py-3 rounded-lg border border-border hover:border-primary hover:bg-primary/5 text-left transition-colors"
      use:autofocus
      on:click={() => onChoose("user")}
    >
      <User size={18} class="text-primary shrink-0 mt-0.5" />
      <span>
        <span class="block text-[0.88rem] font-semibold text-text-bright">{$_("softwareTile.scopeUserTitle")}</span>
        <span class="block text-[0.75rem] text-text-muted">{$_("softwareTile.scopeUserHint")}</span>
      </span>
    </button>
    <button
      class="flex items-start gap-3 px-3.5 py-3 rounded-lg border border-border hover:border-primary hover:bg-primary/5 text-left transition-colors"
      on:click={() => onChoose("system")}
    >
      <Users size={18} class="text-primary shrink-0 mt-0.5" />
      <span>
        <span class="block text-[0.88rem] font-semibold text-text-bright">{$_("softwareTile.scopeSystemTitle")}</span>
        <span class="block text-[0.75rem] text-text-muted">{$_("softwareTile.scopeSystemHint")}</span>
      </span>
    </button>
  </div>
</Modal>
