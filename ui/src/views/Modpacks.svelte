<script lang="ts">
  import { api } from '../lib/api';
  import { app } from '../lib/state.svelte';
  import type { Profile } from '../lib/types';
  let { p }: { p: Profile } = $props();
  // svelte-ignore state_referenced_locally
  let id = $state(p.modpack ?? '');
  $effect(() => { id = p.modpack ?? ''; });
  async function save() {
    const q = $state.snapshot(p) as Profile;
    q.modpack = id.trim() || null;
    const r = await app.guard(() => api.updateProfile(q), 'Modpack del perfil actualizado');
    if (r) await app.refreshProfiles();
  }
</script>

<h1>Modpacks — {p.name}</h1>
<div class="alert warn">Instalar, exportar e importar modpacks Java está <strong>pendiente</strong>. Aquí solo se asocia un id de modpack al perfil (validado: exige un loader fabric/forge/neoforge, y los loaders de DrakesJava están «previstos, no implementados»).</div>
<div class="card stack">
  <div class="field"><label for="mp">Id de modpack</label><input id="mp" bind:value={id} /></div>
  <p class="muted">Loader actual: <strong>{p.loader}</strong>. {p.loader === 'vanilla' ? 'Con loader vanilla el perfil rechazará cualquier modpack.' : ''}</p>
  <button class="btn primary" style="align-self:start" onclick={save}>Guardar</button>
</div>
