<script lang="ts">
  import { api } from '../lib/api';
  import { app } from '../lib/state.svelte';
  import { brand, modeLabel, modeHome } from '../lib/brand';
  import { router } from '../lib/router.svelte';
  import ProfileSummary from '../components/ProfileSummary.svelte';
  import StatePill from '../components/StatePill.svelte';
  import CyberHud from '../components/CyberHud.svelte';
  import type { JavaCheck, ServerStatus, Profile } from '../lib/types';

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
        let st = await api.rustStatus(p.id).catch(() => null);
        if (!st || !st.ready) {
          app.notify('info', 'Instalando y verificando cliente nativo DrakesRust...');
          st = await api.rustInstall(p.id);
        }
        await api.rustStart(p.id);
        app.notify('ok', `⚔️ ¡Cliente DrakesRust conectado a ${p.server || 'mc.drakescraft.cl:25565'}!`);
      } catch (err: any) {
        app.notify('err', `Error al iniciar DrakesRust: ${err?.message || err}`);
      } finally {
        setTimeout(() => { launching = false; }, 2000);
      }
      return;
    }
    if (!compat?.launch_implemented) { app.notify('info', compat?.launch_note ?? 'Lanzamiento no disponible'); return; }
    const r = await app.guard(() => api.serverStart(p.id), 'Servidor arrancando…');
    if (r) router.go('/forge/console');
  }

  async function quickLaunchLan() {
    let localProf = app.profiles.find((pr) => pr.mode === 'local-server');
    if (!localProf) {
      const newProf: Profile = {
        schema: 1,
        id: 'purpur-slimefun-lan',
        name: 'Purpur Slimefun LAN',
        mode: 'local-server',
        channel: 'stable',
        minecraft: '26.2',
        loader: 'vanilla',
        java: {
          major: 25,
          min_mem_mb: 2048,
          max_mem_mb: 4096,
          extra_jvm_args: ['-XX:+UseG1GC', '-Dfile.encoding=UTF-8'],
        },
        server: 'drakespurpur.jar',
        preset: 'slimefun-classic',
        modpack: null,
        notes: 'Instancia LAN Solitario/Multijugador local con Slimefun v11 + StarSuites',
      };
      await app.guard(() => api.createProfile(newProf));
      await api.acceptEula(newProf.id).catch(() => {});
      localProf = newProf;
    }
    const profId = localProf.id;
    await app.setActive(profId);

    // Comprobar si el JAR del servidor ya está instalado
    const srv = await api.serverStatus(profId).catch(() => null);
    if (!srv?.jar?.selected) {
      app.notify('info', 'Descargando y verificando motor DrakesPurpur 26.2 (SHA-256)...');
      await app.guard(() => api.installServerJar(profId), 'Motor DrakesPurpur instalado con éxito');
    }

    // Asegurar EULA aceptada
    await api.acceptEula(profId).catch(() => {});

    app.notify('ok', '⚡ Instancia Purpur + StarSuites iniciada. Abriendo consola Blackwall...');
    const r = await app.guard(() => api.serverStart(profId), 'Servidor local iniciado');
    if (r) {
      router.go('/forge/console');
    } else {
      router.go('/forge');
    }
  }
</script>

<div class="row between">
  <h1>Inicio</h1>
</div>

<CyberHud />

{#if !p}
  <div class="card stack">
    <img src={brand.banner} alt="DrakeStar — Java × Rust Launcher" style="width:100%;max-width:640px" />
    <p>No hay perfil activo. Crea uno para empezar o genera un servidor local al vuelo.</p>
    <div class="row">
      <button class="btn primary" onclick={() => router.go('/profiles')}>Ir a Perfiles</button>
      <button class="btn" onclick={quickLaunchLan} style="border-color:var(--cyber-cyan);color:#00f0ff">⚡ Crear Instancia Purpur + Slimefun LAN</button>
    </div>
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

    <!-- LAN Quick Instance Card (DrakesPurpur + StarSuites Slimefun) -->
    <section class="card stack" style="border: 1px solid rgba(0, 240, 255, 0.35); background: linear-gradient(135deg, rgba(8, 16, 26, 0.8), rgba(4, 8, 14, 0.9)); box-shadow: 0 4px 16px rgba(0, 240, 255, 0.08);">
      <header>
        <span style="font-size: 1.5rem; text-shadow: 0 0 8px #00f0ff;">⚡</span>
        <div class="t">
          <h2 style="margin:0; color: #00f0ff; font-size: 1.05rem; font-family: monospace;">INSTANCIA SLIMEFUN LOCAL (SOLITARIO / LAN)</h2>
          <span class="muted" style="font-size:0.75rem;">DrakesPurpur 26.2 × StarSuites · Slimefun v11 sin configurar nada</span>
        </div>
      </header>
      <p style="font-size:0.85rem; color:#8b949e; margin:0.25rem 0; line-height: 1.4;">
        Genera y arranca al vuelo una instancia local con motor <strong>DrakesPurpur 26.2</strong> y el ecosistema completo de <strong>StarSuites & Slimefun v11</strong>. Conexión directa en solitario o para jugar con amigos en tu red local (Java o Bedrock).
      </p>
      
      <div class="instruction-box" style="background: rgba(0, 0, 0, 0.4); border: 1px dashed rgba(0, 240, 255, 0.25); border-radius: 8px; padding: 0.75rem; font-size: 0.8rem; color: #c9d1d9;">
        <strong style="color: #00f0ff;">📖 Instrucciones de Juego:</strong>
        <ol style="margin: 0.35rem 0 0.5rem 1.2rem; padding: 0; line-height: 1.5;">
          <li>Haz clic en <strong>▶ INICIAR SERVIDOR LOCAL + CONSOLA</strong>. El launcher descargará y verificará automáticamente el JAR oficial (SHA-256) si no está instalado.</li>
          <li>Aceptará el EULA automáticamente y abrirá la terminal Blackwall con telemetría de TPS y memoria.</li>
          <li>En tu cliente de Minecraft (Java 26.2 o 1.21.x), conéctate a <code>localhost:25565</code> o <code>127.0.0.1:25565</code>.</li>
          <li>¡Listo para jugar! Usa <code>/sf guide</code> in-game para acceder al libro y tecnologías de Slimefun.</li>
        </ol>
      </div>

      <div class="row" style="margin-top: 0.25rem;">
        <button class="btn primary" onclick={quickLaunchLan} style="background: linear-gradient(135deg, #00f0ff, #0077b6); border:none; font-weight:bold; color:#05070a; letter-spacing:0.04em;">
          ▶ INICIAR SERVIDOR LOCAL + CONSOLA
        </button>
        <button class="btn" onclick={() => router.go('/forge/plugins')} style="font-size:0.8rem;">
          📦 Ver Plugins / Suites
        </button>
      </div>
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
        <li>Local Forge: arrancar/parar el JAR, consola flotante desacoplable y logs en vivo.</li>
        <li>Descarga con SHA-256 desde un manifiesto importado.</li>
        <li><strong>Disponible:</strong> Lanzar DrakesRust directo a mc.drakescraft.cl:25565, Local Forge, perfiles.</li>
        <li><strong>En desarrollo:</strong> Clientes DrakesJava Fabric/loaders (v0.4).</li>
      </ul>
    </section>

    <!-- Discord Community Card -->
    <section class="card stack" style="border: 1px solid rgba(88, 101, 242, 0.4); background: linear-gradient(135deg, rgba(18, 20, 38, 0.9), rgba(12, 14, 28, 0.95)); box-shadow: 0 4px 18px rgba(88, 101, 242, 0.12);">
      <header>
        <img src={brand.icon('discord')} alt="Discord" width="30" height="30" />
        <div class="t">
          <h2 style="margin:0; color:#9fa8ff; font-size:1.05rem;">COMUNIDAD DRAKESGRAFT</h2>
          <span class="muted" style="font-size:0.75rem;">Soporte, anuncios, guías e integración SRE</span>
        </div>
      </header>
      <p style="font-size:0.85rem; color:#8b949e; margin:0.25rem 0; line-height:1.4;">
        Únete a nuestro Discord oficial soberano. Accede a canales bilingües, avisos de mantenimiento, changelogs en vivo y asistencia directa de SAORI.
      </p>
      <div class="row">
        <a class="btn primary" href={brand.discordUrl} target="_blank" rel="noopener noreferrer" style="background: linear-gradient(135deg, #5865f2, #4752c4); color:#fff; border:none; font-weight:bold; text-decoration:none; display:inline-flex; align-items:center; gap:0.5rem;">
          <img src={brand.icon('discord')} alt="" width="18" height="18" />
          UNIRSE A DISCORD
        </a>
      </div>
    </section>
  </div>
{/if}