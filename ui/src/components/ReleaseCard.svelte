<script lang="ts">
  import { api } from '../lib/api';
  import { app } from '../lib/state.svelte';
  import { bytes } from '../lib/format';
  import type { Profile, Resolution } from '../lib/types';
  import ChannelPill from './ChannelPill.svelte';

  let { profile, installable = false, oninstalled }: { profile: Profile; installable?: boolean; oninstalled?: () => void } = $props();
  let res = $state<Resolution | null>(null);
  let busy = $state(false);

  export async function refresh() {
    res = (await app.guard(() => api.resolveRelease(profile.id))) ?? null;
  }
  $effect(() => { void profile.id, profile.channel, profile.minecraft; void refresh(); });

  async function install() {
    busy = true;
    const n = await app.guard(() => api.installServerJar(profile.id), 'JAR descargado y verificado (SHA-256)');
    busy = false;
    if (n !== undefined) oninstalled?.();
  }
</script>

<div class="stack">
  <div class="row">
    <span class="muted">Canal del perfil:</span> <ChannelPill channel={profile.channel} />
    <span class="muted">· componente <code>{res?.component ?? '—'}</code> · plataforma <code>{res?.platform ?? '—'}</code> · MC <code>{profile.minecraft}</code></span>
  </div>
  {#if res?.resolved}
    {@const r = res.resolved}
    <dl class="kv">
      <dt>Versión</dt><dd><strong>{r.version}</strong> <ChannelPill channel={r.release_channel} /></dd>
      <dt>URL</dt><dd><code>{r.url}</code></dd>
      <dt>SHA-256</dt><dd><code>{r.sha256}</code> {#if !r.sha256_wellformed}<span class="badge no">malformado: no instalable</span>{/if}</dd>
      <dt>Tamaño</dt><dd>{r.size ? bytes(r.size) : 'desconocido'}</dd>
      {#if r.notes}<dt>Notas</dt><dd>{r.notes}</dd>{/if}
    </dl>
    {#if installable}
      <div class="row">
        <button class="btn primary" onclick={install} disabled={busy || !r.sha256_wellformed}>{busy ? 'Descargando…' : 'Descargar y verificar JAR'}</button>
        <span class="hint">Solo https; se escribe a <code>.part</code> y se renombra únicamente si el SHA-256 coincide.</span>
      </div>
    {/if}
  {:else if res}
    <div class="alert warn">Sin release resoluble: {res.reason}</div>
  {/if}
</div>
