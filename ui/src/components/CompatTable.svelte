<script lang="ts">
  import type { Mode } from '../lib/types';
  import { app } from '../lib/state.svelte';
  import CompatBadge from './CompatBadge.svelte';
  let { mode }: { mode: Mode } = $props();
  const c = $derived(app.compatOf(mode));
</script>

{#if c}
  <div class="tablewrap">
    <table>
      <caption class="sr-only">Compatibilidades de {c.label}</caption>
      <thead><tr><th scope="col">Capacidad</th><th scope="col">Estado</th></tr></thead>
      <tbody>
        {#each c.loaders.filter((l) => l.loader !== 'vanilla') as l (l.loader)}
          <tr><th scope="row">Mods {l.loader}</th><td><CompatBadge cell={l.cell} /></td></tr>
        {/each}
        <tr><th scope="row">Modpacks Java</th><td>{#if c.modpacks_allowed}<span class="badge planned">previsto</span>{:else}<span class="badge no">no</span>{/if}</td></tr>
        <tr><th scope="row">Plugins de servidor (Slimefun…)</th><td><CompatBadge cell={c.server_plugins} /></td></tr>
        <tr><th scope="row">GUIs/ítems de Slimefun</th><td><CompatBadge cell={c.slimefun_gui} /></td></tr>
        <tr><th scope="row">Lanzamiento desde DrakeStar</th><td>{#if c.launch_implemented}<span class="badge yes">implementado</span>{:else}<span class="badge no">no disponible</span>{/if}<div class="hint">{c.launch_note}</div></td></tr>
      </tbody>
    </table>
  </div>
{/if}
