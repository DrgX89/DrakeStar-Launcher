<script lang="ts">
  import { brand } from '../lib/brand';
  import { app } from '../lib/state.svelte';
  import ReleaseCard from '../components/ReleaseCard.svelte';
  import ProfileSummary from '../components/ProfileSummary.svelte';
  import CompatTable from '../components/CompatTable.svelte';
  import { api } from '../lib/api';
  import type { Profile, RustClientStatus } from '../lib/types';
  let { p }: { p: Profile } = $props();

  let rustStatus = $state<RustClientStatus | null>(null);
  let busy = $state(false);

  $effect(() => {
    void p.id;
    void api.rustStatus(p.id).then((st) => (rustStatus = st)).catch(() => {});
  });

  async function launchDirect() {
    busy = true;
    try {
      let st = rustStatus;
      if (!st || !st.ready) {
        app.notify('info', 'Descargando y verificando cliente DrakesRust...');
        st = await api.rustInstall(p.id);
        rustStatus = st;
      }
      await api.rustStart(p.id);
      app.notify('ok', `⚔️ ¡Cliente DrakesRust conectado a ${p.server || 'mc.drakescraft.cl:25565'}!`);
    } catch (err: any) {
      app.notify('err', `Error al iniciar DrakesRust: ${err?.message || err}`);
    } finally {
      busy = false;
    }
  }
</script>

<div class="row between"><h1>DrakesRust — {p.name}</h1><img class="pill-img" src={brand.experimental} alt="experimental" /></div>
<div class="alert ok" style="border-left: 4px solid var(--gold); background: rgba(245, 185, 66, 0.08); color: var(--gold-2); display: flex; align-items: center; justify-content: space-between; gap: 1rem; margin-bottom: 1.5rem;">
  <div>
    <strong>⚡ Cliente DrakesRust 26.2 (Rust Engine)</strong> — Listo para arrancar y conectar directamente al SMP oficial de DrakesCraft T2 (<code>mc.drakescraft.cl:25565</code>).
  </div>
  <button class="btn primary" onclick={launchDirect} disabled={busy} style="white-space: nowrap; font-weight: 800; letter-spacing: 1px;">
    {busy ? '⚔️ ARRANCANDO…' : '⚔️ JUGAR AHORA'}
  </button>
</div>
<div class="grid" style="grid-template-columns:repeat(auto-fit,minmax(340px,1fr))">
  <section class="card stack"><h2>Perfil</h2><ProfileSummary {p} />
    <p class="hint">Un perfil DrakesRust corre en runtime nativo con aceleración por hardware (regla de <code>drakestar-core</code>).</p></section>
  <section class="card stack"><h2>Release del cliente</h2><ReleaseCard profile={p} />
    <p class="hint">Motor de juego: DrakesRust 26.2 compilado en Rust nativo (WGPU + Vulkan/DirectX12).</p></section>
  <section class="card stack"><h2>Compatibilidades</h2><CompatTable mode="drakes-rust" /></section>
</div>
