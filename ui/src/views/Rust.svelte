<script lang="ts">
  import { brand } from '../lib/brand';
  import { app } from '../lib/state.svelte';
  import ReleaseCard from '../components/ReleaseCard.svelte';
  import ProfileSummary from '../components/ProfileSummary.svelte';
  import CompatTable from '../components/CompatTable.svelte';
  import type { Profile } from '../lib/types';
  let { p }: { p: Profile } = $props();

  async function launchDirect() {
    try {
      await fetch('/api/launch-rust', { method: 'POST' });
      app.notify('ok', '⚔️ ¡Lanzando DrakesRust conectado a mc.drakescraft.cl:25565!');
    } catch {
      app.notify('info', 'Abriendo lanzador...');
      try { await fetch('/api/launch-horizon', { method: 'POST' }); } catch {}
    }
  }
</script>

<div class="row between"><h1>DrakesRust — {p.name}</h1><img class="pill-img" src={brand.experimental} alt="experimental" /></div>
<div class="alert ok" style="border-left: 4px solid var(--gold); background: rgba(245, 185, 66, 0.08); color: var(--gold-2); display: flex; align-items: center; justify-content: space-between; gap: 1rem; margin-bottom: 1.5rem;">
  <div>
    <strong>⚡ Cliente DrakesRust 26.2 (Rust Engine)</strong> — Listo para arrancar y conectar directamente al SMP oficial de DrakesCraft T2 (<code>mc.drakescraft.cl:25565</code>).
  </div>
  <button class="btn primary" onclick={launchDirect} style="white-space: nowrap; font-weight: 800; letter-spacing: 1px;">⚔️ JUGAR AHORA</button>
</div>
<div class="grid" style="grid-template-columns:repeat(auto-fit,minmax(340px,1fr))">
  <section class="card stack"><h2>Perfil</h2><ProfileSummary {p} />
    <p class="hint">Un perfil DrakesRust corre en runtime nativo con aceleración por hardware (regla de <code>drakestar-core</code>).</p></section>
  <section class="card stack"><h2>Release del cliente</h2><ReleaseCard profile={p} />
    <p class="hint">Motor de juego: DrakesRust 26.2 compilado en Rust nativo (WGPU + Vulkan/DirectX12).</p></section>
  <section class="card stack"><h2>Compatibilidades</h2><CompatTable mode="drakes-rust" /></section>
</div>
