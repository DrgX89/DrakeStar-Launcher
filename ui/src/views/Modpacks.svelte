<script lang="ts">
  import { api } from '../lib/api';
  import { app } from '../lib/state.svelte';
  import type { Profile, DeltaSyncPlan, DeltaSyncResult, ConflictReport } from '../lib/types';
  let { p }: { p: Profile } = $props();
  // svelte-ignore state_referenced_locally
  let id = $state(p.modpack ?? '');
  let manifestJson = $state('');
  let deltaPlan = $state<DeltaSyncPlan | null>(null);
  let deltaResult = $state<DeltaSyncResult | null>(null);
  let conflictReport = $state<ConflictReport | null>(null);

  $effect(() => { id = p.modpack ?? ''; });

  async function save() {
    const q = $state.snapshot(p) as Profile;
    q.modpack = id.trim() || null;
    const r = await app.guard(() => api.updateProfile(q), 'Modpack del perfil actualizado');
    if (r) await app.refreshProfiles();
  }

  async function analyzeAndPlan() {
    if (!manifestJson.trim()) return;
    conflictReport = (await app.guard(() => api.analyzeFabricConflicts(manifestJson))) ?? null;
    deltaPlan = (await app.guard(() => api.modpackDeltaPlan(p.id, manifestJson))) ?? null;
  }

  async function applySync() {
    if (!manifestJson.trim()) return;
    deltaResult = (await app.guard(() => api.modpackDeltaApply(p.id, manifestJson), 'Sincronización diferencial completada')) ?? null;
    deltaPlan = (await app.guard(() => api.modpackDeltaPlan(p.id, manifestJson))) ?? null;
  }
</script>

<h1>Modpacks & Delta-Sync — {p.name}</h1>

<div class="card stack">
  <h3>1. Asociación de Modpack del Perfil</h3>
  <div class="field"><label for="mp">Id de modpack</label><input id="mp" bind:value={id} /></div>
  <p class="muted">Loader actual: <strong>{p.loader}</strong>. {p.loader === 'vanilla' ? 'Con loader vanilla el perfil rechazará cualquier modpack.' : ''}</p>
  <button class="btn primary" style="align-self:start" onclick={save}>Guardar</button>
</div>

<div class="card stack" style="margin-top: 1rem;">
  <h3>2. Sincronización Diferencial Servidor ↔ Cliente & Analizador Fabric/Mixin (Rust)</h3>
  <p class="muted">
    Pega el manifiesto de mods del servidor (<code>ServerModManifest</code> schema 1). Rust compara los hashes SHA-256 de <code>mods/</code>,
    detecta conflictos de dependencias y colisiones de Mixins antes de lanzar la JVM, descarga únicamente los <code>.jar</code> nuevos/modificados y limpia los obsoletos.
  </p>
  <div class="field">
    <label for="manifest-json">Manifiesto JSON del servidor</label>
    <textarea id="manifest-json" rows="6" bind:value={manifestJson} placeholder={`{"schema":1,"server_name":"DrakesCraft T2","minecraft":"26.2","loader":"fabric","mods":[]}`}></textarea>
  </div>
  <div style="display:flex; gap:0.5rem;">
    <button class="btn" onclick={analyzeAndPlan}>Analizar Conflictos & Calcular Delta</button>
    <button class="btn primary" disabled={!deltaPlan || (conflictReport !== null && !conflictReport.compatible)} onclick={applySync}>
      Aplicar Delta-Sync
    </button>
  </div>

  {#if conflictReport}
    <div class="alert {conflictReport.compatible ? 'ok' : 'warn'}">
      <strong>Analizador Pre-Arranque Fabric/Mixin ({conflictReport.mods_analyzed} mods):</strong>
      {conflictReport.compatible ? 'Compatible — sin conflictos bloqueantes.' : 'Se detectaron conflictos antes de arrancar la JVM:'}
      {#if conflictReport.missing_dependencies.length > 0}
        <div>Faltan dependencias: {conflictReport.missing_dependencies.join('; ')}</div>
      {/if}
      {#if conflictReport.explicit_conflicts.length > 0}
        <div>Incompatibilidades: {conflictReport.explicit_conflicts.join('; ')}</div>
      {/if}
      {#if conflictReport.mixin_collisions.length > 0}
        <div>Colisiones de Mixin: {conflictReport.mixin_collisions.join('; ')}</div>
      {/if}
    </div>
  {/if}

  {#if deltaPlan}
    <div class="muted">
      <strong>Plan Diferencial:</strong>
      {deltaPlan.unchanged.length} sin cambios ·
      {deltaPlan.to_download.length} por descargar ({deltaPlan.total_download_bytes} B) ·
      {deltaPlan.to_remove.length} obsoletos por eliminar
    </div>
  {/if}

  {#if deltaResult}
    <div class="alert ok">
      Sincronizados: {deltaResult.downloaded.length} descargados ({deltaResult.bytes_downloaded} B), {deltaResult.removed.length} obsoletos eliminados, {deltaResult.unchanged.length} conservados intactos.
    </div>
  {/if}
</div>
