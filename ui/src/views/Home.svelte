<script lang="ts">
  import { api } from '../lib/api';
  import { app } from '../lib/state.svelte';
  import { brand, modeLabel, modeHome } from '../lib/brand';
  import { router } from '../lib/router.svelte';
  import ProfileSummary from '../components/ProfileSummary.svelte';
  import StatePill from '../components/StatePill.svelte';
  import CyberHud from '../components/CyberHud.svelte';
  import type { JavaCheck, ServerStatus } from '../lib/types';

  const p = $derived(app.active);
  const compat = $derived(p ? app.compatOf(p.mode) : undefined);
  let java = $state<JavaCheck | null>(null);
  let srv = $state<ServerStatus | null>(null);
  let launching = $state(false);

  $effect(() => {
    java = null; srv = null;
    const cur = p;
    if (cur?.mode === 'local-server') {
      void api.javaCheck(cur.id).then((j) => (java = j)).catch(() => {});
      void api.serverStatus(cur.id).then((s) => (srv = s)).catch(() => {});
    }
  });

  async function play() {
    if (!p) return;
    if (p.mode === 'drakes-rust') {
      launching = true;
      try {
        await fetch('/api/launch-rust', { method: 'POST' });
        app.notify('ok', '⚔️ ¡Lanzando DrakesRust conectado a mc.drakescraft.cl:25565!');
      } catch (e) {
        app.notify('info', 'Abriendo lanzador DrakesRust...');
        try { await fetch('/api/launch-horizon', { method: 'POST' }); } catch {}
      } finally {
        setTimeout(() => { launching = false; }, 3000);
      }
      return;
    }
    if (!compat?.launch_implemented) { app.notify('info', compat?.launch_note ?? 'Lanzamiento no disponible'); return; }
    const r = await app.guard(() => api.serverStart(p.id), 'Servidor arrancando…');
    if (r) router.go('/forge/console');
  }
</script>

<div class="row between">
  <h1>Inicio</h1>
</div>

<CyberHud />

{#if !p}
  <div class="card stack">
    <img src={brand.banner} alt="DrakeStar — Java × Rust Launcher" style="width:100%;max-width:640px" />
    <p>No hay perfil activo. Crea uno para empezar.</p>
    <button class="btn primary" onclick={() => router.go('/profiles')}>Ir a Perfiles</button>
  </div>
{:else}
  <div class="grid" style="grid-template-columns:repeat(auto-fit,minmax(320px,1fr))">
    <section class="card stack" aria-labelledby="h-active">
      <header>
        <img class="mi" src={brand.modeIcon(p.mode)} alt="" />
        <div class="t"><h2 id="h-active" style="margin:0">{p.name}</h2><span class="muted mono">{p.id}</span></div>
        {#if p.mode === 'local-server'}<StatePill state={app.statusOf(p.id).state} />{/if}
      </header>
      <ProfileSummary {p} />
      <div class="row">
        <button class="play" onclick={play} aria-label={p.mode === 'drakes-rust' ? 'Lanzar DrakesRust (conectar a mc.drakescraft.cl)' : (compat?.launch_implemented ? 'Iniciar servidor local' : 'Lanzar (no disponible para este modo)')} style="cursor: pointer;">
          <img src="./brand/ui/button-play-primary.svg" alt="JUGAR" height="52" />
        </button>
        <button class="btn" onclick={() => router.go(modeHome[p.mode])}>Abrir {modeLabel[p.mode]}</button>
      </div>
      {#if p.mode === 'drakes-rust'}
        <div class="alert ok" style="border-left: 3px solid var(--gold); background: rgba(245, 185, 66, 0.08); color: var(--gold-2); font-size: 0.9rem;">
          ⚔️ <strong>Servidor Destino:</strong> mc.drakescraft.cl:25565 · Rust Native Client
        </div>
      {:else if compat && !compat.launch_implemented}
        <div class="alert warn">Lanzar no está disponible para {modeLabel[p.mode]}: {compat.launch_note}.</div>
      {/if}
    </section>

    {#if p.mode === 'local-server'}
      <section class="card stack" aria-labelledby="h-comp">
        <h2 id="h-comp">Estado de componentes</h2>
        <dl class="kv">
          <dt>Java</dt>
          <dd>{#if !java}comprobando…{:else if java.info.found}<span class="badge yes">{java.info.version}</span>{:else}<span class="badge no">no encontrado</span>{/if}</dd>
          <dt>JAR del servidor</dt>
          <dd>{#if !srv}…{:else if srv.jar.selected}<span class="badge yes">{srv.jar.selected}</span>{:else}<span class="badge no">falta</span>{/if}</dd>
          <dt>EULA</dt>
          <dd>{#if !srv}…{:else if srv.eula}<span class="badge yes">aceptada</span>{:else}<span class="badge planned">sin aceptar</span>{/if}</dd>
          <dt>Plugins activos</dt><dd>{srv?.plugins ?? '…'}</dd>
          <dt>Mundos</dt><dd>{srv?.worlds ?? '…'}</dd>
        </dl>
        {#if java?.warning}<div class="alert warn">{java.warning}</div>{/if}
      </section>
    {/if}

    <section class="card stack" aria-labelledby="h-state">
      <h2 id="h-state">Qué funciona hoy</h2>
      <ul class="muted" style="padding-left:1.1rem;margin:0">
        <li>Perfiles: CRUD, validación, import/export, persistencia en disco.</li>
        <li>Local Forge: arrancar/parar el JAR, consola y logs en vivo.</li>
        <li>Descarga con SHA-256 desde un manifiesto importado.</li>
        <li><strong>Disponible:</strong> Lanzar DrakesRust directo a mc.drakescraft.cl:25565, Local Forge, perfiles.</li>
        <li><strong>En desarrollo:</strong> Clientes DrakesJava Fabric/loaders (v0.4).</li>
      </ul>
    </section>
  </div>
{/if}
