<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../lib/api';
  import { app } from '../lib/state.svelte';
  import { brand } from '../lib/brand';
  import { router } from '../lib/router.svelte';
  import type { McStatus } from '../lib/types';

  const DALLAS_ADDR = 'mc.drakescraft.cl:25565';
  let status = $state<McStatus | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let connecting = $state(false);

  // Formateador simple de MOTD con codigos § y links
  function formatMotd(raw: string | null | undefined): string {
    if (!raw) return 'Servidor DrakesCraft Dallas SMP';
    // Reemplazar saltos de línea
    let clean = raw.replace(/\\n/g, '<br/>');
    // Convertir codigos de formato básicos §
    clean = clean.replace(/§[0-9a-fk-or]/gi, '');
    return clean;
  }

  // Detección de whitelist a partir de MOTD o raw_json
  const isWhitelist = $derived.by(() => {
    if (!status) return false;
    const txt = (status.description || '') + (status.raw_json || '');
    return txt.toLowerCase().includes('whitelist') || txt.toLowerCase().includes('mantenimiento');
  });

  async function refresh() {
    loading = true;
    error = null;
    try {
      status = await api.pingMcStatus(DALLAS_ADDR);
    } catch (e: any) {
      error = e?.message || 'Servidor no alcanzable';
      status = null;
    } finally {
      loading = false;
    }
  }

  async function connectRust() {
    connecting = true;
    try {
      // Buscar o crear perfil DrakesRust
      let rustProf = app.profiles.find((pr) => pr.mode === 'drakes-rust');
      if (!rustProf) {
        rustProf = {
          schema: 1,
          id: 'drakes-rust-dallas',
          name: 'DrakesCraft Dallas (Rust Client)',
          mode: 'drakes-rust',
          channel: 'stable',
          minecraft: '26.2',
          loader: 'vanilla',
          java: {
            major: 25,
            min_mem_mb: 2048,
            max_mem_mb: 4096,
            extra_jvm_args: [],
          },
          server: DALLAS_ADDR,
          preset: null,
          modpack: null,
          notes: 'Cliente nativo Rust de alto rendimiento para Dallas SMP',
        };
        await app.guard(() => api.createProfile(rustProf!));
      }
      await app.setActive(rustProf.id);

      let st = await api.rustStatus(rustProf.id).catch(() => null);
      if (!st || !st.ready) {
        app.notify('info', 'Instalando y verificando cliente nativo DrakesRust...');
        st = await api.rustInstall(rustProf.id);
      }
      await api.rustStart(rustProf.id);
      app.notify('ok', `⚔️ ¡Conectando cliente DrakesRust a ${DALLAS_ADDR}!`);
    } catch (err: any) {
      app.notify('err', `Error al conectar DrakesRust: ${err?.message || err}`);
    } finally {
      setTimeout(() => { connecting = false; }, 2000);
    }
  }

  onMount(() => {
    void refresh();
    const timer = setInterval(refresh, 20000);
    return () => clearInterval(timer);
  });
</script>

<section class="card stack dallas-card">
  <header class="dallas-header">
    <div class="server-avatar">
      {#if status?.favicon}
        <img src={status.favicon} alt="DrakesCraft Favicon" class="favicon-img" />
      {:else}
        <div class="favicon-fallback">🐉</div>
      {/if}
      <span class="status-indicator" class:online={status !== null} class:offline={status === null && !loading}></span>
    </div>

    <div class="t">
      <div class="title-row">
        <h2 style="margin:0; font-size: 1.15rem; color: var(--gold-2); font-family: monospace;">
          DRAKESCRAFT SMP (TEMPORADA 2)
        </h2>
        {#if status}
          {#if isWhitelist}
            <span class="badge warn" title="Acceso restringido o en mantenimiento">🔒 WHITELIST</span>
          {:else}
            <span class="badge yes">🟢 EN LÍNEA</span>
          {/if}
        {:else if loading}
          <span class="badge planned">⏳ COMPROBANDO...</span>
        {:else}
          <span class="badge no">🔴 DESCONECTADO</span>
        {/if}
      </div>
      <div class="host-pill mono">
        <span>{DALLAS_ADDR}</span>
        {#if status}
          <span class="dot">·</span>
          <span class="latency" class:fast={status.latency_ms < 120} class:med={status.latency_ms >= 120 && status.latency_ms < 220}>
            ⚡ {status.latency_ms} ms
          </span>
          <span class="dot">·</span>
          <span class="version-badge">{status.version_name || 'Purpur 26.2'}</span>
        {/if}
      </div>
    </div>

    <div class="header-actions">
      <button class="btn icon-btn" onclick={refresh} disabled={loading} title="Actualizar estado de Dallas">
        🔄
      </button>
    </div>
  </header>

  <!-- MOTD & Players Display -->
  <div class="dallas-body">
    <div class="motd-box">
      <div class="motd-content">
        {#if status}
          {@html formatMotd(status.description)}
        {:else if loading}
          <span class="muted">Sincronizando estado con el clúster de Dallas...</span>
        {:else}
          <span class="text-danger">No se pudo contactar a mc.drakescraft.cl:25565 ({error})</span>
        {/if}
      </div>
    </div>

    <div class="dallas-stats-row">
      <div class="stat-pill">
        <span class="stat-label">JUGADORES</span>
        <span class="stat-value highlight">
          {status?.players_online ?? 0} <span class="stat-max">/ {status?.players_max ?? 2027}</span>
        </span>
      </div>
      <div class="stat-pill">
        <span class="stat-label">MODALIDAD</span>
        <span class="stat-value">SURVIVAL RPG</span>
      </div>
      <div class="stat-pill">
        <span class="stat-label">UBICACIÓN</span>
        <span class="stat-value">DALLAS, TX 🇺🇸</span>
      </div>
    </div>
  </div>

  <!-- Botones de Acción -->
  <div class="row between wrap actions-row">
    <div class="row">
      <button 
        class="btn primary play-dallas-btn" 
        onclick={connectRust} 
        disabled={connecting}
        style="background: linear-gradient(135deg, var(--gold), #d97706); color: #000; font-weight: bold; border: none; box-shadow: 0 0 14px rgba(245, 185, 66, 0.4);"
      >
        ⚔️ CONECTAR CON RUST NATIVE
      </button>
      <button 
        class="btn" 
        onclick={() => router.go('/profiles')}
        style="font-size: 0.85rem;"
      >
        ☕ Conectar con Java Client
      </button>
    </div>

    <div class="row">
      <a 
        class="btn discord-link-btn" 
        href={brand.discordUrl} 
        target="_blank" 
        rel="noopener noreferrer"
      >
        <img src={brand.icon('discord')} alt="" width="16" height="16" />
        Discord Oficial
      </a>
    </div>
  </div>
</section>

<style>
  .dallas-card {
    border: 1px solid rgba(245, 185, 66, 0.35);
    background: linear-gradient(135deg, rgba(20, 16, 10, 0.9) 0%, rgba(12, 10, 6, 0.95) 100%);
    box-shadow: 0 4px 20px rgba(245, 185, 66, 0.1);
  }

  .dallas-header {
    display: flex;
    align-items: center;
    gap: 0.85rem;
  }

  .server-avatar {
    position: relative;
    width: 48px;
    height: 48px;
    border-radius: 8px;
    background: #111;
    border: 1px solid rgba(245, 185, 66, 0.4);
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: visible;
  }

  .favicon-img {
    width: 100%;
    height: 100%;
    border-radius: 7px;
    image-rendering: pixelated;
  }

  .favicon-fallback {
    font-size: 1.6rem;
  }

  .status-indicator {
    position: absolute;
    bottom: -3px;
    right: -3px;
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: #6b7280;
    border: 2px solid #000;
  }

  .status-indicator.online {
    background: #10b981;
    box-shadow: 0 0 8px #10b981;
  }

  .status-indicator.offline {
    background: #ef4444;
  }

  .title-row {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    flex-wrap: wrap;
  }

  .host-pill {
    font-size: 0.75rem;
    color: #9ca3af;
    display: flex;
    align-items: center;
    gap: 0.4rem;
    margin-top: 0.15rem;
  }

  .dot {
    color: #4b5563;
  }

  .latency.fast {
    color: #10b981;
  }

  .latency.med {
    color: #f59e0b;
  }

  .version-badge {
    background: rgba(255, 255, 255, 0.08);
    padding: 0.1rem 0.35rem;
    border-radius: 4px;
    font-size: 0.7rem;
    color: #e5e7eb;
  }

  .header-actions {
    margin-left: auto;
  }

  .icon-btn {
    padding: 0.3rem 0.6rem;
    font-size: 0.85rem;
  }

  .dallas-body {
    display: flex;
    flex-direction: column;
    gap: 0.65rem;
  }

  .motd-box {
    background: rgba(0, 0, 0, 0.55);
    border: 1px dashed rgba(245, 185, 66, 0.25);
    border-radius: 8px;
    padding: 0.65rem 0.85rem;
    font-family: monospace;
    font-size: 0.82rem;
    line-height: 1.45;
    color: #f3f4f6;
    text-shadow: 1px 1px 2px rgba(0, 0, 0, 0.8);
  }

  .dallas-stats-row {
    display: flex;
    gap: 0.75rem;
    flex-wrap: wrap;
  }

  .stat-pill {
    flex: 1;
    min-width: 120px;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 6px;
    padding: 0.4rem 0.6rem;
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
  }

  .stat-label {
    font-size: 0.65rem;
    color: #9ca3af;
    letter-spacing: 0.05em;
  }

  .stat-value {
    font-size: 0.85rem;
    font-weight: 600;
    color: #e5e7eb;
  }

  .stat-value.highlight {
    color: #10b981;
  }

  .stat-max {
    font-size: 0.75rem;
    color: #6b7280;
    font-weight: normal;
  }

  .actions-row {
    margin-top: 0.35rem;
  }

  .discord-link-btn {
    background: rgba(88, 101, 242, 0.15);
    border: 1px solid rgba(88, 101, 242, 0.4);
    color: #9fa8ff;
    font-size: 0.8rem;
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    text-decoration: none;
    padding: 0.45rem 0.75rem;
    border-radius: 6px;
  }

  .discord-link-btn:hover {
    background: rgba(88, 101, 242, 0.3);
    color: #fff;
  }

  .text-danger {
    color: #ef4444;
  }
</style>
