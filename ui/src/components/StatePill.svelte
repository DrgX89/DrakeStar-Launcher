<script lang="ts">
  import type { ProcState } from '../lib/types';
  import { brand, stateLabel } from '../lib/brand';
  let { state }: { state: ProcState } = $props();
  const src = $derived(brand.statePill(state));
</script>

{#if src}
  <img class="pill-img" {src} alt={stateLabel[state]} />
{:else}
  <span class="badge stopping" role="status">{stateLabel[state].toUpperCase()}</span>
{/if}
