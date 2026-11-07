<script lang="ts">
  import { api } from '../lib/api';
  import { app } from '../lib/state.svelte';
  import ProfileSummary from '../components/ProfileSummary.svelte';
  import CompatTable from '../components/CompatTable.svelte';
  import type { Profile } from '../lib/types';
  let { p }: { p: Profile } = $props();
  let result = $state<Record<string, { ms?: number; err?: string; busy?: boolean }>>({});
  const online = $derived(app.profiles.filter((x) => x.mode === 'online'));
  async function ping(q: Profile) {
    result[q.id] = { busy: true };
    try { result[q.id] = { ms: await api.ping(q.server ?? '') }; } catch (e) { result[q.id] = { err: e instanceof Error ? e.message : String(e) }; }
  }
</script>

<h1>Online — {p.name}</h1>
<div class="alert info">El «ping» es una <strong>conexión TCP</strong> al puerto (por defecto 25565): indica si el puerto responde y en cuánto tiempo, no la versión ni los jugadores. Solo se ejecuta al pulsar el botón. Conectar al servidor requiere un cliente (DrakesJava/DrakesRust): pendiente.</div>
<div class="grid" style="grid-template-columns:repeat(auto-fit,minmax(340px,1fr))">
  <section class="card stack"><h2>Perfil activo</h2><ProfileSummary {p} /></section>
  <section class="card stack"><h2>Accesos rápidos</h2>
    {#each online as q (q.id)}
      <div class="row between">
        <div><strong>{q.name}</strong><div class="mono muted">{q.server}</div></div>
        <div class="row">
          {#if result[q.id]?.busy}<span class="muted">comprobando…</span>
          {:else if result[q.id]?.ms !== undefined}<span class="badge yes">{result[q.id].ms} ms</span>
          {:else if result[q.id]?.err}<span class="badge no" title={result[q.id].err}>sin respuesta</span>{/if}
          <button class="btn small" onclick={() => ping(q)}>Ping TCP</button>
        </div>
      </div>
      {#if result[q.id]?.err}<div class="hint">{result[q.id].err}</div>{/if}
    {/each}
  </section>
  <section class="card stack"><h2>Compatibilidades</h2><CompatTable mode="online" /></section>
</div>
