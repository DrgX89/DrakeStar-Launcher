<script lang="ts">
  import { api } from '../lib/api';
  import { app } from '../lib/state.svelte';
  import { bytes, when } from '../lib/format';
  import type { Profile, WorldInfo } from '../lib/types';
  let { p }: { p: Profile } = $props();
  let list = $state<WorldInfo[]>([]);
  $effect(() => { void p.id; void app.guard(() => api.worlds(p.id)).then((r) => (list = r ?? [])); });
</script>

<h1>Mundos — {p.name}</h1>
<p class="muted">Detecta carpetas con <code>level.dat</code> en el directorio del servidor. Crear/importar/respaldar mundos está <strong>pendiente</strong> (solo listado por ahora).</p>
<div class="card tablewrap">
  <table>
    <thead><tr><th scope="col">Mundo</th><th scope="col">Tamaño</th><th scope="col">Modificado</th></tr></thead>
    <tbody>
      {#each list as w (w.name)}<tr><td><strong>{w.name}</strong></td><td>{bytes(w.size)}</td><td>{when(w.modified)}</td></tr>
      {:else}<tr><td colspan="3" class="muted">Aún no hay mundos (se generan al arrancar el servidor).</td></tr>{/each}
    </tbody>
  </table>
</div>
