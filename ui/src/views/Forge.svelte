<script lang="ts">
  import { api } from '../lib/api';
  import { app } from '../lib/state.svelte';
  import { brand } from '../lib/brand';
  import { router } from '../lib/router.svelte';
  import ReleaseCard from '../components/ReleaseCard.svelte';
  import StatePill from '../components/StatePill.svelte';
  import ProfileSummary from '../components/ProfileSummary.svelte';
  import type { JavaCheck, Profile, Prop, ServerStatus } from '../lib/types';

  let { p }: { p: Profile } = $props();
  let srv = $state<ServerStatus | null>(null);
  let java = $state<JavaCheck | null>(null);
  let plan = $state<string | null>(null);
  let planErr = $state<string | null>(null);
  let sprops = $state<Prop[]>([]);
  let edit = $state<Record<string, string>>({});
  let eulaOk = $state(false);

  const st = $derived(app.statusOf(p.id));
  const active = $derived(st.state === 'running' || st.state === 'stopping');
  const KEYS: { key: string; label: string; type?: 'number' | 'bool' | 'select'; opts?: string[] }[] = [
    { key: 'motd', label: 'MOTD' },
    { key: 'server-port', label: 'Puerto', type: 'number' },
    { key: 'max-players', label: 'Máx. jugadores', type: 'number' },
    { key: 'level-name', label: 'Nombre del mundo' },
    { key: 'difficulty', label: 'Dificultad', type: 'select', opts: ['peaceful', 'easy', 'normal', 'hard'] },
    { key: 'gamemode', label: 'Modo de juego', type: 'select', opts: ['survival', 'creative', 'adventure', 'spectator'] },
    { key: 'view-distance', label: 'Distancia de visión', type: 'number' },
    { key: 'online-mode', label: 'online-mode (cuentas premium)', type: 'bool' },
    { key: 'white-list', label: 'Lista blanca', type: 'bool' },
  ];

  async function load() {
    srv = (await app.guard(() => api.serverStatus(p.id))) ?? null;
    java = (await app.guard(() => api.javaCheck(p.id))) ?? null;
    sprops = (await app.guard(() => api.serverProperties(p.id))) ?? [];
    edit = Object.fromEntries(sprops.map((x) => [x.key, x.value]));
    try { plan = await api.serverPlan(p.id); planErr = null; } catch (e) { plan = null; planErr = e instanceof Error ? e.message : String(e); }
    await app.refreshStatus(p.id);
  }
  $effect(() => { void p.id; void load(); });
  // al terminar el proceso se refresca el estado en disco (eula.txt, server.properties, etc.)
  // svelte-ignore state_referenced_locally
  let prev = st.state;
  $effect(() => { const s = st.state; if (prev !== s && s === 'stopped') void load(); prev = s; });

  async function start() { const r = await app.guard(() => api.serverStart(p.id), 'Servidor arrancando…'); if (r) router.go('/forge/console'); }
  const stop = () => app.guard(() => api.serverStop(p.id), 'Parada solicitada (comando stop)');
  const kill = () => app.guard(() => api.serverKill(p.id), 'Proceso terminado a la fuerza');
  async function accept() { if (await app.guard(() => api.acceptEula(p.id).then(() => true), 'EULA aceptada')) await load(); }
  async function saveProps() {
    const updates: Record<string, string> = {};
    const orig = Object.fromEntries(sprops.map((x) => [x.key, x.value]));
    for (const k of KEYS) if (edit[k.key] !== undefined && edit[k.key] !== orig[k.key]) updates[k.key] = edit[k.key];
    if (!Object.keys(updates).length) { app.notify('info', 'Sin cambios'); return; }
    const r = await app.guard(() => api.setServerProperties(p.id, updates), 'server.properties guardado (aplica al reiniciar)');
    if (r) { sprops = r; edit = Object.fromEntries(r.map((x) => [x.key, x.value])); }
  }
  const hasKey = (k: string) => sprops.some((x) => x.key === k);
  const unknown = $derived(sprops.filter((x) => !KEYS.some((k) => k.key === x.key)));
</script>

<div class="row between">
  <h1>Local Forge</h1>
  <StatePill state={st.state} />
</div>

<div class="grid" style="grid-template-columns:repeat(auto-fit,minmax(340px,1fr))">
  <section class="card stack" aria-labelledby="f-run">
    <h2 id="f-run">Servidor — {p.name}</h2>
    <ProfileSummary {p} />
    <div class="row">
      <button class="btn primary" onclick={start} disabled={active}><img src={brand.icon('play')} alt="" />Iniciar</button>
      <button class="btn" onclick={stop} disabled={st.state !== 'running'}><img src={brand.icon('stop')} alt="" />Detener</button>
      <button class="btn danger" onclick={kill} disabled={!active}>Matar</button>
      <button class="btn" onclick={() => router.go('/forge/console')}><img src={brand.icon('console')} alt="" />Consola</button>
    </div>
    {#if st.state === 'crashed'}<div class="alert err" role="alert">El proceso terminó con código {st.exit_code ?? '?'}. Revisa la consola.</div>{/if}
    {#if java?.warning}<div class="alert warn">{java.warning} Configura la ruta en <button class="btn small" onclick={() => router.go('/settings/about')}>Acerca de</button>.</div>{/if}
    <h3>Plan de lanzamiento (drakestar-core)</h3>
    {#if plan}<pre class="mono card" style="white-space:pre-wrap;margin:0">{plan}</pre>
    {:else if planErr}<div class="alert warn">{planErr}</div>{/if}
  </section>

  <section class="card stack" aria-labelledby="f-files">
    <h2 id="f-files">Carpeta del servidor</h2>
    {#if srv}
      <dl class="kv">
        <dt>Ruta</dt><dd><code>{srv.dir}</code></dd>
        <dt>JAR</dt><dd>{#if srv.jar.selected}<span class="badge yes">{srv.jar.selected}</span>{:else if srv.jar.candidates.length > 1}<span class="badge planned">varios .jar: renombra el correcto a drakespurpur.jar</span>{:else}<span class="badge no">falta</span> coloca <code>drakespurpur.jar</code> aquí o descarga una release{/if}</dd>
        <dt>EULA</dt><dd>{#if srv.eula}<span class="badge yes">aceptada</span>{:else}<span class="badge planned">{srv.eula === null ? 'eula.txt no existe' : 'sin aceptar'}</span>{/if}</dd>
        <dt>Plugins</dt><dd>{srv.plugins} activos · <a href="#/forge/plugins">gestionar</a></dd>
        <dt>Mundos</dt><dd>{srv.worlds} · <a href="#/forge/worlds">ver</a></dd>
      </dl>
      {#if !srv.eula}
        <div class="alert info">
          <label style="color:var(--text)"><input type="checkbox" bind:checked={eulaOk} /> He leído y acepto el <a href="https://aka.ms/MinecraftEULA" target="_blank" rel="noreferrer">EULA de Minecraft</a></label>
          <button class="btn small" onclick={accept} disabled={!eulaOk}>Escribir eula=true</button>
        </div>
      {/if}
    {/if}
  </section>

  <section class="card stack" aria-labelledby="f-rel">
    <h2 id="f-rel">Release del servidor</h2>
    <ReleaseCard profile={p} installable oninstalled={load} />
    <p class="hint">Requiere un manifiesto importado en Actualizaciones. No hay aún un manifiesto oficial publicado.</p>
  </section>

  <section class="card stack" aria-labelledby="f-sprops">
    <h2 id="f-sprops">server.properties</h2>
    {#if !sprops.length}<p class="muted">El archivo aún no existe (lo crea el servidor en el primer arranque). Puedes escribir valores igualmente.</p>{/if}
    <div class="form-grid">
      {#each KEYS as k (k.key)}
        <div class="field">
          <label for={`sp-${k.key}`}>{k.label} <span class="mono">({k.key})</span></label>
          {#if k.type === 'bool'}
            <select id={`sp-${k.key}`} value={edit[k.key] ?? ''} onchange={(e) => (edit[k.key] = e.currentTarget.value)}>
              {#if !hasKey(k.key)}<option value="">(sin definir)</option>{/if}<option value="true">true</option><option value="false">false</option>
            </select>
          {:else if k.type === 'select'}
            <select id={`sp-${k.key}`} value={edit[k.key] ?? ''} onchange={(e) => (edit[k.key] = e.currentTarget.value)}>
              {#if !hasKey(k.key)}<option value="">(sin definir)</option>{/if}{#each k.opts ?? [] as o (o)}<option value={o}>{o}</option>{/each}
            </select>
          {:else}
            <input id={`sp-${k.key}`} type={k.type === 'number' ? 'number' : 'text'} value={edit[k.key] ?? ''} oninput={(e) => (edit[k.key] = e.currentTarget.value)} />
          {/if}
        </div>
      {/each}
    </div>
    <div class="row"><button class="btn primary" onclick={saveProps} disabled={active}>Guardar</button>{#if active}<span class="hint">Detén el servidor para editar.</span>{/if}</div>
    {#if unknown.length}
      <details><summary>{unknown.length} propiedades más (solo lectura)</summary>
        <div class="tablewrap"><table><tbody>{#each unknown as u (u.key)}<tr><th scope="row" class="mono">{u.key}</th><td class="mono">{u.value}</td></tr>{/each}</tbody></table></div>
      </details>
    {/if}
  </section>
</div>
