<script lang="ts">
  import { api } from '../lib/api';
  import { app } from '../lib/state.svelte';
  import { bytes } from '../lib/format';
  import type { LoadReport, Profile } from '../lib/types';

  let { p }: { p: Profile } = $props();
  let report = $state<LoadReport | null>(null);
  let busy = $state(false);
  let err = $state<string | null>(null);

  async function show() {
    const r = await app.guard(() => api.modsReport(p.id));
    if (r) { report = r; err = null; } else err = 'no se pudo leer el estado de los mods';
  }
  // decidir = cargar de verdad: verifica y deshabilita lo que no carga
  async function decide() {
    busy = true;
    const r = await app.guard(() => api.modsDecide(p.id));
    busy = false;
    if (r) { report = r; err = null; }
  }
  $effect(() => { void p.id; void show(); });

  // mapa id -> runtime/abi para mostrarlo junto al estado
  const meta = $derived(new Map((report?.abi ?? []).map(([id, rt, abi]) => [id, { rt, abi }])));
  const orderIndex = $derived(new Map((report?.order ?? []).map((id, i) => [id, i + 1])));
  const notLoaded = $derived(new Map(report?.not_loaded ?? []));
  const allIds = $derived([...new Set([...(report?.order ?? []), ...notLoaded.keys()])].sort());
</script>

<div class="row between">
  <h1>Mods — {p.name}</h1>
  <button class="btn primary" onclick={decide} disabled={busy}>{busy ? 'Verificando…' : 'Verificar y aplicar'}</button>
</div>

<div class="alert info">
  El cargador es Rust: descubre los jars (incluidos los que viajan dentro de otros),
  verifica el <strong>SHA-256</strong> de cada uno contra <code>mods.lock.json</code>,
  valida el ABI y los hooks con el intermediario, y resuelve el grafo de dependencias.
  Lo que no carga queda como <code>.jar.disabled</code> (reversible). Quien ejecuta los
  mods sigue siendo Fabric Loader: lo que cambia es <strong>quién decide</strong>.
</div>

{#if err}<div class="alert err" role="alert">{err}</div>{/if}

{#if report}
  <div class="row" style="gap:1.5rem;flex-wrap:wrap">
    <span class="badge neutral">{report.discovered} descubiertos</span>
    <span class="badge {report.verified === report.discovered ? 'yes' : 'planned'}">{report.verified} verificados (SHA-256)</span>
    <span class="badge {report.order.length ? 'yes' : 'no'}">{report.order.length} cargan</span>
    {#if report.not_loaded.length}<span class="badge no">{report.not_loaded.length} no cargan</span>{/if}
    <span class="muted">{bytes(report.bytes)} en classpath</span>
  </div>

  {#if report.skipped.length}
    <details style="margin-top:.5rem"><summary>{report.skipped.length} jar(s) que no son mods de Fabric válidos</summary>
      <ul class="muted">{#each report.skipped as s (s)}<li class="mono">{s}</li>{/each}</ul>
    </details>
  {/if}

  <div class="card tablewrap" style="margin-top:.75rem">
    <table>
      <thead><tr><th scope="col">#</th><th scope="col">Mod</th><th scope="col">Runtime</th><th scope="col">ABI</th><th scope="col">Estado</th></tr></thead>
      <tbody>
        {#each allIds as id (id)}
          {@const m = meta.get(id)}
          {@const pos = orderIndex.get(id)}
          {@const why = notLoaded.get(id)}
          <tr>
            <td class="muted mono">{pos ?? '—'}</td>
            <td><strong>{id}</strong></td>
            <td>{m?.rt ?? '—'}</td>
            <td class="mono">{m?.abi ?? '—'}</td>
            <td>
              {#if why}
                <span class="badge no">no carga</span> <span class="muted">{why}</span>
              {:else}
                <span class="badge yes">carga</span>
              {/if}
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>

  {#if report.warnings.length}
    <div class="card stack" style="margin-top:.75rem">
      <h2>Avisos</h2>
      <ul class="muted">{#each report.warnings as w (w[0] + w[1])}<li><strong>{w[0]}</strong>: {w[1]}</li>{/each}</ul>
    </div>
  {/if}
{:else if !err}
  <p class="muted" role="status">Cargando…</p>
{/if}
