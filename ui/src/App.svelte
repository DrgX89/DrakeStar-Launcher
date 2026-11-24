<script lang="ts">
  import { onMount } from 'svelte';
  import Icon from './components/Icon.svelte';
  import MsaModal from './components/MsaModal.svelte';
  import Toasts from './components/Toasts.svelte';
  import Welcome from './components/Welcome.svelte';
  import { api } from './lib/api';
  import { errText } from './lib/format';
  import { t } from './lib/i18n.svelte';
  import { app, type View } from './lib/store.svelte';
  import Catalog from './views/Catalog.svelte';
  import Accounts from './views/Accounts.svelte';
  import Instances from './views/Instances.svelte';
  import Logs from './views/Logs.svelte';
  import Play from './views/Play.svelte';
  import Settings from './views/Settings.svelte';

  const nav: { id: View; icon: string }[] = [
    { id: 'play', icon: 'play' },
    { id: 'accounts', icon: 'user' },
    { id: 'instances', icon: 'box' },
    { id: 'catalog', icon: 'download' },
    { id: 'settings', icon: 'sliders' },
    { id: 'logs', icon: 'file' },
  ];

  onMount(() => { app.init(); });
</script>

<div class="app">
  <aside class="side">
    <div class="brand"><img src="./brand/isotype.svg" alt="" /><span>DrakeStar</span></div>
    <nav class="stack" style="gap:4px" aria-label="DrakeStar">
      {#each nav as n (n.id)}
        <button type="button" class="navbtn" class:on={app.view === n.id} aria-current={app.view === n.id ? 'page' : undefined} title={t('nav.' + n.id)} onclick={() => (app.view = n.id)}>
          <Icon name={n.icon} /><span>{t('nav.' + n.id)}</span>
        </button>
      {/each}
    </nav>
    <div class="side-foot">
      v{app.info?.version ?? ''}<br />Minecraft {app.info?.min_mc ?? '1.20'} – {app.info?.max_mc ?? '26.2'}
    </div>
  </aside>

  <main>
    {#if app.update}
      <div class="banner" role="status">
        <Icon name="download" />
        <span class="grow">{t('upd.banner', { v: app.update.version })}</span>
        {#if app.busy}<span class="small muted">{t('upd.busy')}</span>{/if}
        <button class="btn sm primary" type="button" disabled={app.busy || app.updateState === 'installing'} onclick={() => app.installUpdate()}>{app.updateState === 'installing' ? t('set.installing') : t('upd.install')}</button>
      </div>
    {/if}

    {#if app.ready}
      {#if app.instancesError && (app.view === 'play' || app.view === 'instances')}
        <div class="banner" role="alert" style="align-items:flex-start">
          <Icon name="alert" />
          <div class="stack grow" style="gap:8px; min-width:0">
            <strong>{t('ins.load_error')}</strong>
            <p class="small">{t('ins.load_error_hint')}</p>
            <p class="small" style="overflow-wrap:anywhere">{app.instancesError}</p>
            <div class="row">
              <button class="btn sm" type="button" disabled={app.instancesLoading} onclick={() => app.loadInstances()}>{t('common.retry')}</button>
              <button class="btn sm" type="button" onclick={() => api.openFolder('data').catch((e) => app.toast('error', errText(e)))}><Icon name="folder" /> {t('set.open_data')}</button>
            </div>
          </div>
        </div>
      {/if}
      {#if app.view === 'play'}<Play />
      {:else if app.view === 'accounts'}<Accounts />
      {:else if app.view === 'instances'}<Instances />
      {:else if app.view === 'catalog'}<Catalog />
      {:else if app.view === 'settings'}<Settings />
      {:else}<Logs />{/if}
    {/if}
  </main>
</div>

<MsaModal />
<Welcome />
<Toasts />
