<script lang="ts">
  import { api } from '../lib/api';
  import { app } from '../lib/state.svelte';
  import { brand, modeLabel } from '../lib/brand';
  import { bytes, when } from '../lib/format';
  import StatePill from '../components/StatePill.svelte';
  import ChannelPill from '../components/ChannelPill.svelte';
  import type { InstanceRow, LogFile, Profile } from '../lib/types';

  let rows = $state<InstanceRow[]>([]);
  let loading = $state(true);
  let logsFor = $state<{ p: Profile; files: LogFile[]; open?: { name: string; lines: string[] } } | null>(null);

  async function load() { rows = (await app.guard(() => api.listInstances())) ?? []; loading = false; }
  $effect(() => { void load(); });
  $effect(() => { void JSON.stringify(app.status); void load(); });

  async function create(p: Profile) { if (await app.guard(() => api.ensureInstance(p.id).then(() => true), 'Estructura creada')) await load(); }
  async function showLogs(p: Profile) { const files = await app.guard(() => api.listLogs(p.id)); if (files) logsFor = { p, files }; }
  async function openLog(f: LogFile) {
    if (!logsFor) return;
    const lines = await app.guard(() => api.readLog(logsFor!.p.id, f.source, f.name, 400));
    if (lines) logsFor = { ...logsFor, open: { name: `${f.source}/${f.name}`, lines } };
  }
  async function clear() {
    if (!logsFor) return;
    const n = await app.guard(() => api.clearLogs(logsFor!.p.id));
    if (n !== undefined) { app.notify('ok', `${n} log(s) del launcher borrados`); await showLogs(logsFor.p); await load(); }
  }
</script>

<h1>Instancias</h1>
<p class="muted">Una instancia es el directorio de datos de un perfil: la carpeta del servidor (Local Forge) o del juego (clientes), más los logs que captura DrakeStar. Los perfiles Online no tienen datos locales.</p>

{#if loading}<p class="muted">Cargando…</p>{/if}
<div class="tablewrap card">
  <table>
    <thead><tr><th scope="col">Perfil</th><th scope="col">Canal</th><th scope="col">Datos</th><th scope="col">Tamaño</th><th scope="col">Estado</th><th scope="col">Acciones</th></tr></thead>
    <tbody>
      {#each rows as r (r.profile.id)}
        <tr>
          <td><img src={brand.modeIcon(r.profile.mode)} alt="" width="22" height="22" style="vertical-align:middle" /> <strong>{r.profile.name}</strong><div class="muted mono">{modeLabel[r.profile.mode]} · {r.profile.id}</div></td>
          <td><ChannelPill channel={r.profile.channel} /></td>
          <td>
            {#if r.profile.mode === 'online'}<span class="muted">—</span>
            {:else if r.info.data_exists}<code>{r.info.data_dir}</code>{#if r.profile.mode === 'local-server'}<div class="muted">JAR: {r.info.server_jar ?? 'falta'}</div>{/if}
            {:else}<span class="badge planned">sin crear</span>{/if}
          </td>
          <td>{bytes(r.info.size_bytes)}<div class="muted">mod. {when(r.info.modified)}</div></td>
          <td>{#if r.profile.mode === 'local-server'}<StatePill state={r.status.state} />{:else}<span class="muted">—</span>{/if}</td>
          <td>
            <div class="row">
              {#if r.profile.mode !== 'online' && !r.info.data_exists}<button class="btn small" onclick={() => create(r.profile)}>Crear carpetas</button>{/if}
              <button class="btn small" onclick={() => showLogs(r.profile)}>Logs ({r.info.launcher_logs})</button>
            </div>
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
</div>

{#if logsFor}
  <section class="card stack" style="margin-top:1rem" aria-labelledby="h-logs">
    <div class="row between"><h2 id="h-logs">Logs de {logsFor.p.name}</h2><div class="row"><button class="btn small danger" onclick={clear}>Borrar logs del launcher</button><button class="btn small" onclick={() => (logsFor = null)}>Cerrar</button></div></div>
    {#if logsFor.files.length === 0}<p class="muted">Sin archivos de log.</p>{/if}
    <ul style="margin:0;padding-left:1.1rem">
      {#each logsFor.files as f (f.source + f.name)}
        <li><button class="btn small" onclick={() => openLog(f)}>{f.source}/{f.name}</button> <span class="muted">{bytes(f.size)} · {when(f.modified)}</span></li>
      {/each}
    </ul>
    {#if logsFor.open}
      <h3>{logsFor.open.name} (últimas {logsFor.open.lines.length} líneas)</h3>
      <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
      <pre class="console" tabindex="0">{logsFor.open.lines.join('\n')}</pre>
    {/if}
  </section>
{/if}
