<script lang="ts">
  import { brand } from '../lib/brand';
  import ReleaseCard from '../components/ReleaseCard.svelte';
  import ProfileSummary from '../components/ProfileSummary.svelte';
  import CompatTable from '../components/CompatTable.svelte';
  import type { Profile } from '../lib/types';
  let { p }: { p: Profile } = $props();
</script>

<div class="row between"><h1>DrakesRust — {p.name}</h1><img class="pill-img" src={brand.experimental} alt="experimental" /></div>
<div class="alert warn">No existe todavía un cliente Rust empaquetado. DrakeStar gestiona el perfil y resuelve releases por canal, pero <strong>no puede lanzar el cliente</strong>. Que un cliente Rust (p. ej. basado en Horizon) conecte a DrakesPurpur 26.2 y muestre las GUIs de Slimefun <strong>no está verificado</strong>.</div>
<div class="grid" style="grid-template-columns:repeat(auto-fit,minmax(340px,1fr))">
  <section class="card stack"><h2>Perfil</h2><ProfileSummary {p} />
    <p class="hint">Un perfil DrakesRust no acepta loaders ni modpacks Java (regla de <code>drakestar-core</code>).</p></section>
  <section class="card stack"><h2>Release del cliente</h2><ReleaseCard profile={p} />
    <p class="hint">Descarga del cliente: pendiente (no hay artefactos publicados). LOD y diagnóstico: pendientes.</p></section>
  <section class="card stack"><h2>Compatibilidades</h2><CompatTable mode="drakes-rust" /></section>
</div>
