<script lang="ts">
  import { api } from '../lib/api';
  import { app } from '../lib/state.svelte';
  import { brand, modeLabel, modeHome } from '../lib/brand';
  import { router } from '../lib/router.svelte';
  import ProfileSummary from '../components/ProfileSummary.svelte';
  import StatePill from '../components/StatePill.svelte';
  import type { JavaCheck, ServerStatus } from '../lib/types';

  const p = $derived(app.active);
  const compat = $derived(p ? app.compatOf(p.mode) : undefined);
  let java = $state<JavaCheck | null>(null);
  let srv = $state<ServerStatus | null>(null);

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
    if (!compat?.launch_implemented) { app.notify('info', compat?.launch_note ?? 'Lanzamiento no disponible'); return; }
    const r = await app.guard(() => api.serverStart(p.id), 'Servidor arrancando…');
    if (r) router.go('/forge/console');
  }
</script>

<div class="row between">
  <h1>Inicio</h1>
</div>

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
        <button class="play" onclick={play} aria-label={compat?.launch_implemented ? 'Iniciar servidor local' : 'Lanzar (no disponible para este modo)'} aria-disabled={!compat?.launch_implemented} class:off={!compat?.launch_implemented}>
          <img src="./brand/ui/button-play-primary.svg" alt="" height="52" />
        </button>
        <button class="btn" onclick={() => router.go(modeHome[p.mode])}>Abrir {modeLabel[p.mode]}</button>
      </div>
      {#if compat && !compat.launch_implemented}
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
        <li><strong>No disponible:</strong> lanzar clientes DrakesJava/DrakesRust, instalar loaders/modpacks, JRE gestionado.</li>
      </ul>
    </section>
  </div>
{/if}

<style>
  .play { background: none; border: 0; padding: 0; cursor: pointer; line-height: 0; border-radius: 10px; }
  .play.off { filter: grayscale(.8) brightness(.6); cursor: not-allowed; }
  .play:hover:not(.off) { filter: brightness(1.12); }
</style>
