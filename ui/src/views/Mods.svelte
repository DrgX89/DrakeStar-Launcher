<script lang="ts">
  import { api } from '../lib/api';
  import { app } from '../lib/state.svelte';
  import { bytes } from '../lib/format';
  import { playClick, playSuccess, playError } from '../lib/sound';
  import type { LoadReport, Profile, ModrinthSearchHit, ScanResult } from '../lib/types';

  let { p }: { p: Profile } = $props();
  let report = $state<LoadReport | null>(null);
  let busy = $state(false);
  let err = $state<string | null>(null);

  // Modrinth Modal State
  let showModrinth = $state(false);
  let modrinthQuery = $state('');
  let modrinthHits = $state<ModrinthSearchHit[]>([]);
  let modrinthLoading = $state(false);
  let modrinthInstallStatus = $state<string | null>(null);

  async function show() {
    const r = await app.guard(() => api.modsReport(p.id));
    if (r) { report = r; err = null; } else err = 'no se pudo leer el estado de los mods';
  }

  // decidir = cargar de verdad: verifica y deshabilita lo que no carga
  async function decide() {
    playClick();
    busy = true;
    const r = await app.guard(() => api.modsDecide(p.id));
    busy = false;
    if (r) {
      report = r;
      err = null;
      playSuccess();
    } else {
      playError();
    }
  }

  async function toggleGuardOverride(file: string, currentAllowed: boolean) {
    playClick();
    const newAllowed = !currentAllowed;
    await api.modsGuardOverride(p.id, file, newAllowed);
    await decide();
  }

  async function searchModrinth() {
    if (!modrinthQuery.trim()) return;
    playClick();
    modrinthLoading = true;
    modrinthInstallStatus = null;
    try {
      modrinthHits = await api.modrinthSearch(modrinthQuery.trim(), 12);
    } catch (e) {
      modrinthInstallStatus = `Error buscando mods: ${e}`;
    } finally {
      modrinthLoading = false;
    }
  }

  async function installFromModrinth(hit: ModrinthSearchHit) {
    playClick();
    modrinthInstallStatus = `Descargando ${hit.title}…`;
    try {
      const versions = await api.modrinthVersions(hit.project_id, p.minecraft);
      if (!versions || versions.length === 0) {
        modrinthInstallStatus = `No se encontró versión compatible para Minecraft ${p.minecraft}`;
        playError();
        return;
      }
      const primaryFile = versions[0].files.find((f) => f.primary) ?? versions[0].files[0];
      if (!primaryFile) {
        modrinthInstallStatus = 'No se encontró archivo de mod descargable';
        playError();
        return;
      }
      await api.modrinthInstall(p.id, primaryFile.url, primaryFile.filename);
      modrinthInstallStatus = `¡${hit.title} instalado exitosamente!`;
      playSuccess();
      await show();
    } catch (e) {
      modrinthInstallStatus = `Fallo al instalar: ${e}`;
      playError();
    }
  }

  $effect(() => { void p.id; void show(); });

  // mapa id -> runtime/abi para mostrarlo junto al estado
  const meta = $derived(new Map((report?.abi ?? []).map(([id, rt, abi]) => [id, { rt, abi }])));
  const orderIndex = $derived(new Map((report?.order ?? []).map((id, i) => [id, i + 1])));
  const notLoaded = $derived(new Map(report?.not_loaded ?? []));
  const allIds = $derived([...new Set([...(report?.order ?? []), ...notLoaded.keys()])].sort());
  const securityAlerts = $derived(report?.security_alerts ?? []);
</script>

<div class="row between wrap" style="gap:1rem;margin-bottom:1rem">
  <div>
    <h1>Mods — {p.name}</h1>
    <p class="muted">Gestión de dependencias, protección de integridad y catálogo Modrinth</p>
  </div>
  <div class="row" style="gap:.75rem">
    <button class="btn secondary" onclick={() => { playClick(); showModrinth = !showModrinth; }}>
      📦 {showModrinth ? 'Cerrar Catálogo' : 'Explorar Modrinth'}
    </button>
    <button class="btn primary" onclick={decide} disabled={busy}>
      {busy ? 'Verificando…' : '⚡ Verificar y aplicar'}
    </button>
  </div>
</div>

<!-- Modal / Drawer de Modrinth -->
{#if showModrinth}
  <div class="card stack" style="margin-bottom:1.5rem;border:1px solid var(--accent);background:rgba(20,20,30,0.85)">
    <div class="row between">
      <h2>🌐 Catálogo Oficial Modrinth (1-Click Install)</h2>
      <button class="btn sub" onclick={() => { playClick(); showModrinth = false; }}>✕</button>
    </div>
    <p class="muted">Busca e instala mods oficiales de Fabric directamente para Minecraft {p.minecraft}.</p>
    
    <div class="row" style="gap:.5rem">
      <input
        type="text"
        placeholder="Buscar mods en Modrinth (ej. sodium, iris, lithium, ferritecore)…"
        bind:value={modrinthQuery}
        onkeydown={(e) => e.key === 'Enter' && searchModrinth()}
        style="flex:1"
      />
      <button class="btn primary" onclick={searchModrinth} disabled={modrinthLoading}>
        {modrinthLoading ? 'Buscando…' : 'Buscar'}
      </button>
    </div>

    {#if modrinthInstallStatus}
      <div class="alert info">{modrinthInstallStatus}</div>
    {/if}

    {#if modrinthHits.length > 0}
      <div class="grid" style="grid-template-columns:repeat(auto-fill, minmax(280px, 1fr));gap:1rem;margin-top:.75rem">
        {#each modrinthHits as hit (hit.project_id)}
          <div class="card stack" style="background:rgba(255,255,255,0.03);border:1px solid rgba(255,255,255,0.1)">
            <div class="row between">
              <strong>{hit.title}</strong>
              <span class="badge neutral">{hit.downloads.toLocaleString()} descargas</span>
            </div>
            <p class="muted" style="font-size:0.85rem;line-height:1.3;flex:1">{hit.description}</p>
            <div class="row between" style="margin-top:.5rem">
              <span class="muted mono" style="font-size:0.75rem">por {hit.author}</span>
              <button class="btn primary" style="font-size:0.85rem;padding:.3rem .75rem" onclick={() => installFromModrinth(hit)}>
                ⬇️ Instalar
              </button>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </div>
{/if}

<!-- DrakeStar Blackwall Guard Sentinel Card -->
<div class="card stack" style="margin-bottom:1.25rem;border-left:4px solid {securityAlerts.length > 0 ? '#ef4444' : '#10b981'}">
  <div class="row between">
    <div class="row" style="gap:.75rem;align-items:center">
      <span style="font-size:1.4rem">{securityAlerts.length > 0 ? '🛡️⚠️' : '🛡️'}</span>
      <div>
        <h3 style="margin:0">DrakeStar Blackwall Guard</h3>
        <p class="muted" style="margin:0;font-size:0.85rem">
          {securityAlerts.length > 0
            ? 'Atención: Se han detectado llamadas riesgosas o patrones no autorizados en mods.'
            : 'Sistema de defensa activo: 100% limpio. Sin robo de llaves SSH, sin malware ni backdoors.'}
        </p>
      </div>
    </div>
    <span class="badge {securityAlerts.length > 0 ? 'no' : 'yes'}">
      {securityAlerts.length > 0 ? `${securityAlerts.length} Alerta(s)` : 'Seguro'}
    </span>
  </div>

  {#if securityAlerts.length > 0}
    <div class="stack" style="gap:.75rem;margin-top:.75rem">
      {#each securityAlerts as alert (alert.file)}
        <div class="card stack" style="background:rgba(239,68,68,0.08);border:1px solid rgba(239,68,68,0.25);padding:1rem">
          <div class="row between wrap">
            <span class="mono" style="color:#fca5a5;font-weight:bold">{alert.file}</span>
            <span class="badge no">{alert.risk.toUpperCase()}</span>
          </div>
          <ul style="margin:.5rem 0;color:#fca5a5;font-size:0.9rem">
            {#each alert.reasons as reason}
              <li>⚠️ {reason}</li>
            {/each}
          </ul>
          <label class="row" style="gap:.5rem;align-items:center;cursor:pointer;margin-top:.25rem;user-select:none">
            <input
              type="checkbox"
              checked={alert.user_override}
              onchange={() => toggleGuardOverride(alert.file, alert.user_override)}
            />
            <span style="font-size:0.88rem;color:{alert.user_override ? '#34d399' : '#e2e8f0'}">
              <strong>Permitir bajo mi responsabilidad:</strong> Sé lo que estoy haciendo y autorizo este archivo en mi servidor/cliente.
            </span>
          </label>
        </div>
      {/each}
    </div>
  {/if}
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
