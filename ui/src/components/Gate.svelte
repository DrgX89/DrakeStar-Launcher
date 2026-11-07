<script lang="ts">
  import type { Mode } from '../lib/types';
  import { app } from '../lib/state.svelte';
  import { modeLabel } from '../lib/brand';
  import { router } from '../lib/router.svelte';
  let { mode }: { mode: Mode } = $props();
  const candidates = $derived(app.profiles.filter((p) => p.mode === mode));
</script>

<div class="card stack">
  <h1>{modeLabel[mode]}</h1>
  <p>Esta página es para perfiles de modo <strong>{modeLabel[mode]}</strong>. El perfil activo{app.active ? ` (${app.active.name})` : ''} es de otro modo.</p>
  {#if candidates.length}
    <div class="row">
      {#each candidates as p (p.id)}
        <button class="btn" onclick={() => app.setActive(p.id)}>Activar «{p.name}»</button>
      {/each}
    </div>
  {:else}
    <p class="muted">No tienes ningún perfil de este modo.</p>
    <button class="btn primary" onclick={() => router.go('/profiles')}>Crear un perfil</button>
  {/if}
</div>
