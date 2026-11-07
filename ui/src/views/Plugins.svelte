<script lang="ts">
  import { api } from '../lib/api';
  import { app } from '../lib/state.svelte';
  import { bytes } from '../lib/format';
  import type { PluginInfo, Profile } from '../lib/types';
  let { p }: { p: Profile } = $props();
  let list = $state<PluginInfo[]>([]);
  const active = $derived(['running', 'stopping'].includes(app.statusOf(p.id).state));
  async function load() { list = (await app.guard(() => api.plugins(p.id))) ?? []; }
  $effect(() => { void p.id; void load(); });
  async function toggle(f: PluginInfo) { if (await app.guard(() => api.togglePlugin(p.id, f.file), f.enabled ? 'Plugin desactivado' : 'Plugin activado')) await load(); }
</script>

<h1>Plugins — {p.name}</h1>
<p class="muted">Lista los <code>.jar</code> de <code>plugins/</code>. Desactivar renombra a <code>.jar.disabled</code> (el servidor debe estar detenido). Copiar JARs a la carpeta es manual por ahora; no hay instalación desde repositorio.</p>
{#if active}<div class="alert warn">Detén el servidor para activar/desactivar plugins.</div>{/if}
<div class="card tablewrap">
  <table>
    <thead><tr><th scope="col">Plugin</th><th scope="col">Archivo</th><th scope="col">Tamaño</th><th scope="col">Estado</th><th scope="col"><span class="sr-only">Acción</span></th></tr></thead>
    <tbody>
      {#each list as f (f.file)}
        <tr>
          <td><strong>{f.name}</strong></td><td class="mono">{f.file}</td><td>{bytes(f.size)}</td>
          <td>{#if f.enabled}<span class="badge yes">activo</span>{:else}<span class="badge planned">desactivado</span>{/if}</td>
          <td><button class="btn small" onclick={() => toggle(f)} disabled={active}>{f.enabled ? 'Desactivar' : 'Activar'}</button></td>
        </tr>
      {:else}
        <tr><td colspan="5" class="muted">No hay plugins en la carpeta.</td></tr>
      {/each}
    </tbody>
  </table>
</div>
