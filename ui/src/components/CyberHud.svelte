<script lang="ts">
  import { onMount } from 'svelte';
  import { app } from '../lib/state.svelte';

  interface TelemetryItem {
    label: string;
    value: string;
    status: 'ok' | 'active' | 'sync' | 'warn';
  }

  let ping = $state<number | null>(null);
  let pulse = $state(true);

  const activeProfile = $derived(app.active);

  const metrics = $derived<TelemetryItem[]>([
    {
      label: 'HYBRID KERNEL',
      value: 'Purpur 26.2 b140 · FFM ABI 1',
      status: 'ok',
    },
    {
      label: 'HORIZON PREFETCH',
      value: 'Active (3.5s Lead · 6s TTL)',
      status: 'active',
    },
    {
      label: 'DELTA MODSYNC',
      value: 'SHA-256 Server-Driven v1',
      status: 'sync',
    },
    {
      label: 'SRE MESH',
      value: 'Star Fleet & SAORI Online',
      status: 'ok',
    },
  ]);

  onMount(() => {
    const timer = setInterval(() => {
      pulse = !pulse;
    }, 2400);
    return () => clearInterval(timer);
  });
</script>

<div class="cyber-hud-container" role="region" aria-label="Telemetría del Motor Híbrido">
  <div class="cyber-scanline" aria-hidden="true"></div>

  <div class="hud-left">
    <div class="ring-visual" aria-hidden="true">
      <svg class="hud-svg" viewBox="0 0 120 120" width="88" height="88">
        <!-- Outer Track Ring -->
        <circle cx="60" cy="60" r="54" class="ring-track" />
        
        <!-- Outer Rotating Dashed Ring -->
        <circle cx="60" cy="60" r="54" class="ring-outer" />
        
        <!-- Middle Counter-Rotating Ring -->
        <circle cx="60" cy="60" r="42" class="ring-middle" />
        
        <!-- Inner Core Ring -->
        <circle cx="60" cy="60" r="30" class="ring-inner" />
        
        <!-- Reticle Crosshairs -->
        <line x1="60" y1="12" x2="60" y2="24" class="reticle-line" />
        <line x1="60" y1="96" x2="60" y2="108" class="reticle-line" />
        <line x1="12" y1="60" x2="24" y2="60" class="reticle-line" />
        <line x1="96" y1="60" x2="108" y2="60" class="reticle-line" />
        
        <!-- Central Glowing Core -->
        <circle cx="60" cy="60" r="14" class="core-glow" />
        <polygon points="60,52 67,64 53,64" class="core-glyph" />
      </svg>
    </div>

    <div class="hud-identity">
      <div class="badge-row">
        <span class="status-dot"></span>
        <span class="hud-deck-title">DRAKESTAR HYBRID DECK</span>
        <span class="hud-channel">SEASON TWO</span>
      </div>
      <div class="hud-sub">
        {#if activeProfile}
          <span class="active-profile-name">{activeProfile.name}</span>
          <span class="muted">({activeProfile.mode})</span>
        {:else}
          <span class="muted">TELEMETRÍA EN ESPERA DE PERFIL</span>
        {/if}
      </div>
    </div>
  </div>

  <div class="hud-telemetry-grid">
    {#each metrics as m (m.label)}
      <div class="telemetry-card">
        <span class="tel-label">{m.label}</span>
        <span class="tel-value {m.status}">
          <span class="mini-pill {m.status}"></span>
          {m.value}
        </span>
      </div>
    {/each}
  </div>
</div>

<style>
  .cyber-hud-container {
    position: relative;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 1.25rem;
    padding: 1rem 1.25rem;
    margin: 0.75rem 0 1.25rem 0;
    background: linear-gradient(135deg, rgba(14, 16, 26, 0.92) 0%, rgba(22, 18, 36, 0.88) 100%);
    border: 1px solid rgba(168, 85, 247, 0.32);
    border-radius: 12px;
    box-shadow: 0 4px 24px rgba(0, 0, 0, 0.45), inset 0 0 16px rgba(168, 85, 247, 0.08);
    overflow: hidden;
  }

  .cyber-scanline {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: repeating-linear-gradient(
      to bottom,
      transparent 0px,
      transparent 3px,
      rgba(255, 255, 255, 0.015) 3px,
      rgba(255, 255, 255, 0.015) 4px
    );
    pointer-events: none;
    z-index: 1;
  }

  .hud-left {
    display: flex;
    align-items: center;
    gap: 1rem;
    z-index: 2;
  }

  .ring-visual {
    display: flex;
    align-items: center;
    justify-content: center;
    filter: drop-shadow(0 0 10px rgba(229, 169, 59, 0.4));
  }

  .hud-svg {
    transform-origin: center;
  }

  .ring-track {
    fill: none;
    stroke: rgba(255, 255, 255, 0.08);
    stroke-width: 1.5;
  }

  .ring-outer {
    fill: none;
    stroke: #e5a93b;
    stroke-width: 2.2;
    stroke-dasharray: 18 12;
    animation: rotate-cw 20s linear infinite;
    transform-origin: 60px 60px;
  }

  .ring-middle {
    fill: none;
    stroke: #a855f7;
    stroke-width: 2;
    stroke-dasharray: 8 6 16 6;
    animation: rotate-ccw 14s linear infinite;
    transform-origin: 60px 60px;
  }

  .ring-inner {
    fill: none;
    stroke: #06b6d4;
    stroke-width: 1.8;
    stroke-dasharray: 32 10;
    animation: rotate-cw 8s linear infinite;
    transform-origin: 60px 60px;
  }

  .reticle-line {
    stroke: rgba(229, 169, 59, 0.6);
    stroke-width: 1.5;
  }

  .core-glow {
    fill: rgba(168, 85, 247, 0.25);
    stroke: #a855f7;
    stroke-width: 1.5;
    animation: core-pulse 3s ease-in-out infinite alternate;
  }

  .core-glyph {
    fill: #e5a93b;
  }

  @keyframes rotate-cw {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }

  @keyframes rotate-ccw {
    from { transform: rotate(0deg); }
    to { transform: rotate(-360deg); }
  }

  @keyframes core-pulse {
    from { r: 12px; fill: rgba(168, 85, 247, 0.15); }
    to { r: 16px; fill: rgba(229, 169, 59, 0.35); }
  }

  .hud-identity {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .badge-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .status-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #10b981;
    box-shadow: 0 0 8px #10b981;
    animation: blink 2s infinite;
  }

  @keyframes blink {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.4; }
  }

  .hud-deck-title {
    font-size: 0.85rem;
    font-weight: 800;
    letter-spacing: 0.08em;
    color: #f1f5f9;
    text-transform: uppercase;
  }

  .hud-channel {
    font-size: 0.65rem;
    font-weight: 700;
    letter-spacing: 0.06em;
    padding: 0.15rem 0.4rem;
    background: rgba(229, 169, 59, 0.2);
    color: #e5a93b;
    border: 1px solid rgba(229, 169, 59, 0.4);
    border-radius: 4px;
  }

  .hud-sub {
    font-size: 0.8rem;
    color: #94a3b8;
  }

  .active-profile-name {
    color: #38bdf8;
    font-weight: 600;
  }

  .hud-telemetry-grid {
    display: flex;
    flex-wrap: wrap;
    gap: 0.75rem;
    z-index: 2;
  }

  .telemetry-card {
    background: rgba(10, 12, 20, 0.6);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 8px;
    padding: 0.4rem 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
    min-width: 140px;
  }

  .tel-label {
    font-size: 0.62rem;
    font-weight: 700;
    color: #94a3b8;
    letter-spacing: 0.05em;
  }

  .tel-value {
    font-size: 0.75rem;
    font-family: ui-monospace, monospace;
    font-weight: 600;
    display: flex;
    align-items: center;
    gap: 0.35rem;
  }

  .mini-pill {
    width: 6px;
    height: 6px;
    border-radius: 50%;
  }

  .tel-value.ok { color: #34d399; }
  .mini-pill.ok { background: #10b981; box-shadow: 0 0 6px #10b981; }

  .tel-value.active { color: #38bdf8; }
  .mini-pill.active { background: #0ea5e9; box-shadow: 0 0 6px #0ea5e9; }

  .tel-value.sync { color: #fbbf24; }
  .mini-pill.sync { background: #f59e0b; box-shadow: 0 0 6px #f59e0b; }

  .tel-value.warn { color: #f87171; }
  .mini-pill.warn { background: #ef4444; box-shadow: 0 0 6px #ef4444; }
</style>
