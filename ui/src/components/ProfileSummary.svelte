<script lang="ts">
  import type { Profile } from '../lib/types';
  import ChannelPill from './ChannelPill.svelte';
  import { modeLabel } from '../lib/brand';
  let { p }: { p: Profile } = $props();
</script>

<dl class="kv">
  <dt>Modo</dt><dd>{modeLabel[p.mode]}</dd>
  <dt>Canal</dt><dd><ChannelPill channel={p.channel} /></dd>
  <dt>Minecraft</dt><dd>{p.minecraft}</dd>
  <dt>Loader</dt><dd>{p.loader}</dd>
  {#if p.modpack}<dt>Modpack</dt><dd>{p.modpack}</dd>{/if}
  {#if p.server}<dt>Servidor</dt><dd>{p.server}</dd>{/if}
  {#if p.preset}<dt>Preset</dt><dd>{p.preset}</dd>{/if}
  <dt>Memoria</dt><dd>{p.java.min_mem_mb}–{p.java.max_mem_mb} MiB{p.java.major ? ` · Java ${p.java.major}` : ''}</dd>
</dl>
