<script lang="ts">
  import { afterUpdate } from "svelte";
  import { terminalLines, isRunning, terminalExpanded } from "../lib/progress";

  export let fullWidth = false;

  let el: HTMLDivElement;
  let wasRunning = false;

  // afterUpdate runs once per DOM patch cycle (not on every store notification
  // like a `$:` reactive block would), which avoids the freeze that a
  // `$: ... queueMicrotask(...)` block tied to this element caused in the VM.
  afterUpdate(() => {
    if (el) {
      el.scrollTop = el.scrollHeight;
      // A plain el.scrollIntoView() walks every scrollable ancestor it finds,
      // which fights with Drivers' own inline scan-results box (it has its
      // own overflow-y-auto list) and made the page jump around. Scrolling
      // the single known page container directly instead is deterministic.
      if ($isRunning && !wasRunning) {
        const container = document.getElementById("main-scroll");
        if (container) {
          const targetTop =
            el.getBoundingClientRect().top - container.getBoundingClientRect().top + container.scrollTop;
          // No "smooth": WebKitGTK's animated scrollTo can crash the
          // WebProcess on VM software rendering.
          container.scrollTo({ top: targetTop });
        }
      }
    }
    wasRunning = $isRunning;
  });
</script>

{#if $terminalExpanded}
  <div
    bind:this={el}
    class="content-scroll w-full {fullWidth ? '' : 'max-w-[700px] mx-auto'} h-[160px] overflow-y-auto overscroll-contain bg-panel border border-border rounded p-3 font-mono text-[0.75rem] text-text leading-relaxed"
  >
    {#each $terminalLines as line}
      <div>{line}</div>
    {/each}
    {#if $isRunning}
      <div class="text-primary">▌</div>
    {/if}
  </div>
{/if}
