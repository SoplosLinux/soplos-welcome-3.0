<script lang="ts">
  export let title: string;
</script>

<header class="grid grid-cols-[1fr_auto_1fr] items-center px-7 py-4 border-b border-border bg-panel shrink-0">
  <div class="flex gap-2.5 justify-self-start">
    <slot name="back" />
  </div>
  <h1 class="text-[1.25rem] font-semibold text-text-bright whitespace-nowrap">{title}</h1>
  <div class="flex gap-2.5 justify-self-end">
    <slot name="actions" />
  </div>
</header>
