<script lang="ts">
  import { tick } from 'svelte';
  import { api } from '../lib/api';
  import { app } from '../lib/state.svelte';
  import { time, bytes, when } from '../lib/format';
  import StatePill from '../components/StatePill.svelte';
  import type { LogFile, Profile } from '../lib/types';

  let { p }: { p: Profile } = $props();
  let cmd = $state('');
  let follow = $state(true);
  let box: HTMLDivElement | undefined = $state();
  let hist: string[] = [];
  let hpos = -1;
  let logs = $state<LogFile[]>([]);
  let opened = $state<{ name: string; lines: string[] } | null>(null);

  const lines = $derived(app.consoles[p.id] ?? []);
  const st = $derived(app.statusOf(p.id));
  const running = $derived(st.state === 'running');

  $effect(() => { void app.loadConsole(p.id); void loadLogs(); });
  $effect(() => {
    void lines.length;
    if (follow) void tick().then(() => { if (box) box.scrollTop = box.scrollHeight; });
  });
  async function loadLogs() { logs = (await app.guard(() => api.listLogs(p.id))) ?? []; }
  async function send() {
    const l = cmd.trim();
    if (!l) return;
    const ok = await app.guard(() => api.serverCommand(p.id, l).then(() => true));
    if (ok) { hist.unshift(l); hpos = -1; cmd = ''; }
  }
  function key(e: KeyboardEvent) {
    if (e.key === 'ArrowUp' && hist.length) { e.preventDefault(); hpos = Math.min(hpos + 1, hist.length - 1); cmd = hist[hpos]; }
    else if (e.key === 'ArrowDown') { e.preventDefault(); hpos = Math.max(hpos - 1, -1); cmd = hpos < 0 ? '' : hist[hpos]; }
  }
  async function openLog(f: LogFile) {
    const l = await app.guard(() => api.readLog(p.id, f.source, f.name, 1000));
    if (l) opened = { name: `${f.source}/${f.name}`, lines: l };
  }
</script>

<div class="row between">
  <h1>Consola — {p.name}</h1>
  <div class="row">
    <StatePill state={st.state} />
    <button class="btn small" onclick={() => api.serverStart(p.id).then(() => app.notify('ok', 'Arrancando…')).catch((e) => app.notify('err', String(e.message ?? e)))} disabled={st.state === 'running' || st.state === 'stopping'}>Iniciar</button>
    <button class="btn small" onclick={() => app.guard(() => api.serverStop(p.id))} disabled={!running}>Detener</button>
  </div>
</div>

<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<div class="console" bind:this={box} role="log" aria-live="off" aria-label="Salida del servidor" tabindex="0">
  {#if lines.length === 0}<span class="muted">Sin salida todavía. Inicia el servidor desde Local Forge.</span>{/if}
  {#each lines as l, i (i)}
    <span class="l {l.stream}"><span class="ts">{time(l.ts)}</span>{l.stream === 'stdin' ? '> ' : ''}{l.line}</span>
  {/each}
</div>

<form class="cmdline" onsubmit={(e) => { e.preventDefault(); void send(); }}>
  <label for="cmd" class="sr-only">Comando para el servidor</label>
  <input id="cmd" bind:value={cmd} onkeydown={key} placeholder={running ? 'Comando (p. ej. list, say hola, stop) — ↑/↓ historial' : 'El servidor no está en ejecución'} disabled={!running} autocomplete="off" spellcheck="false" />
  <button class="btn primary" type="submit" disabled={!running || !cmd.trim()}>Enviar</button>
</form>
<div class="row" style="margin-top:.5rem">
  <label style="color:var(--text);margin:0"><input type="checkbox" bind:checked={follow} /> Seguir salida</label>
  <button class="btn small" onclick={() => app.guard(() => api.consoleClear(p.id).then(() => (app.consoles[p.id] = [])))}>Limpiar vista</button>
  <span class="muted">{lines.length} líneas · log: <code>{st.log_file ?? '—'}</code></span>
</div>

<section class="card stack" style="margin-top:1rem" aria-labelledby="c-logs">
  <div class="row between"><h2 id="c-logs">Archivos de log</h2><button class="btn small" onclick={loadLogs}>Actualizar</button></div>
  {#if logs.length === 0}<p class="muted">Aún no hay logs en disco.</p>{/if}
  <ul style="margin:0;padding-left:1.1rem">
    {#each logs as f (f.source + f.name)}
      <li><button class="btn small" onclick={() => openLog(f)}>{f.source}/{f.name}</button> <span class="muted">{bytes(f.size)} · {when(f.modified)}</span></li>
    {/each}
  </ul>
  {#if opened}<h3>{opened.name}</h3><!-- svelte-ignore a11y_no_noninteractive_tabindex -->
      <pre class="console" tabindex="0">{opened.lines.join('\n')}</pre>{/if}
</section>
