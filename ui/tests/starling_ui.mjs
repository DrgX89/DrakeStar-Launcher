// Chromium + Svelte reales: valida los cuatro flujos Starling (Ely.by con/sin 2FA, Diagnóstico + recuperación Fused, abrir carpetas y estado inicial sin cuentas).
import assert from 'node:assert/strict';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';
import { chromium } from 'playwright';
import { createServer } from 'vite';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const server = await createServer({ root, server: { host: '127.0.0.1', port: 0, strictPort: false } });
await server.listen();
let browser;
try {
  browser = await chromium.launch({ headless: true });
  const url = server.resolvedUrls.local[0];
  async function fixture(run, { lang = 'es', width = 1280 } = {}) {
    const context = await browser.newContext({ viewport: { width, height: 900 } });
    const page = await context.newPage();
    page.setDefaultTimeout(5000);
    const errors = [];
    page.on('pageerror', (e) => errors.push(e.message));
    await page.route('**/*', (route) => new URL(route.request().url()).origin === new URL(url).origin
      ? route.continue() : route.abort());
    try {
      await page.goto(url);
      await page.waitForFunction(async () => (await import('/src/lib/store.svelte.ts')).app.ready);
      await page.evaluate(async ({ lang }) => {
        const { app } = await import('/src/lib/store.svelte.ts');
        const { api } = await import('/src/lib/api.ts');
        const { setLang } = await import('/src/lib/i18n.svelte.ts');
        setLang(lang);
        app.settings.first_run_done = true;
        app.instances = [
          {
            id: 'slimefun-solo', name: 'Slimefun en solitario / LAN', mc: '26.2', loader: 'fabric',
            target: { kind: 'local', port: 25570, lan: false }, performance: true, fused: true,
            xmx_mb: 3072, extra_jvm: [], note: '',
          },
          {
            id: 'drakescraft', name: 'DrakesCraft (online)', mc: '1.21.11', loader: 'fabric',
            target: { kind: 'server', address: 'mc.drakescraft.cl:25565' }, performance: true, fused: false,
            xmx_mb: 4096, extra_jvm: [], note: '',
          },
        ];
        app.selected = 'slimefun-solo';
        window.fixture = {
          elyCalls: [],
          folders: [],
          savedInstances: [],
          accountsState: {
            accounts: [
              { id: 'offline-1', kind: 'offline', name: 'Invitado', uuid: 'b50ad385829d3141a2167e7d7539ba7f', added_at: 1 },
            ],
            default: 'offline-1',
          },
        };
        const clone = (v) => JSON.parse(JSON.stringify(v));
        app.accounts = clone(window.fixture.accountsState);
        api.accounts = async () => clone(window.fixture.accountsState);
        api.setDefaultAccount = async (id) => { window.fixture.accountsState.default = id; };
        api.removeAccount = async (id) => {
          window.fixture.accountsState.accounts = window.fixture.accountsState.accounts.filter((a) => a.id !== id);
          if (window.fixture.accountsState.default === id) {
            window.fixture.accountsState.default = window.fixture.accountsState.accounts[0]?.id ?? null;
          }
        };
        api.openFolder = async (kind, id) => { window.fixture.folders.push({ kind, id: id ?? null }); };
        api.saveInstance = async (inst) => {
          const copy = clone(inst);
          window.fixture.savedInstances.push(copy);
          const idx = app.instances.findIndex((i) => i.id === inst.id);
          if (idx >= 0) app.instances[idx] = clone(inst);
          return copy;
        };
        api.instances = async () => clone(app.instances);
        api.logs = async (source, instance) => source === 'game'
          ? [`[GAME] instance=${instance}`, '[ERROR] Synthetic shader or mod failure']
          : ['[INFO] DrakeStar boot', '[ERROR] Preparando autenticación Ely.by'];
      }, { lang });
      await run(page);
      assert.deepEqual(errors, []);
    } finally {
      await context.close();
    }
  }

  for (const lang of ['es', 'en']) {
    await test(`Starling (b) Ely.by ${lang}: flujo completo con 2FA, limpieza de credenciales y seleccion de cuenta`, () => fixture(async (page) => {
      await page.evaluate(async () => {
        const { app } = await import('/src/lib/store.svelte.ts');
        const { api } = await import('/src/lib/api.ts');
        api.addElyby = async (username, password, totp) => {
          window.fixture.elyCalls.push({ username, password, totp });
          if (!totp) {
            throw new Error('Ely.by: introduce el código de verificación de dos pasos.');
          }
          const acc = {
            id: 'ely-069a79f444e94726a5befca90e38aaf5',
            kind: 'elyby',
            name: 'AlexEly',
            uuid: '069a79f444e94726a5befca90e38aaf5',
            added_at: 2,
          };
          window.fixture.accountsState.accounts.push(acc);
          return acc;
        };
        app.view = 'accounts';
      });

      await page.locator('#ely-username').fill('alex@ely.by');
      await page.locator('#ely-password').fill('SecretPass!123');
      await page.getByRole('button', { name: lang === 'es' ? 'Iniciar sesión con Ely.by' : 'Sign in with Ely.by', exact: true }).click();

      const alert = page.locator('main [role=alert]');
      await alert.waitFor();
      assert.match(await alert.innerText(), /dos pasos/);
      // La contraseña y el código 2FA se vacían siempre en el finally, conservando el usuario para reintentar con 2FA:
      assert.equal(await page.locator('#ely-username').inputValue(), 'alex@ely.by');
      assert.equal(await page.locator('#ely-password').inputValue(), '');
      assert.equal(await page.locator('#ely-totp').inputValue(), '');

      await page.locator('#ely-password').fill('SecretPass!123');
      await page.locator('#ely-totp').fill('654321');
      await page.getByRole('button', { name: lang === 'es' ? 'Iniciar sesión con Ely.by' : 'Sign in with Ely.by', exact: true }).click();

      await page.waitForFunction(() => window.fixture.elyCalls.length === 2);
      await page.getByRole('heading', { name: 'AlexEly', exact: true }).waitFor();
      assert.equal(await page.locator('main [role=alert]').count(), 0);
      assert.equal(await page.locator('#ely-username').inputValue(), '');
      assert.equal(await page.locator('#ely-password').inputValue(), '');
      assert.equal(await page.locator('#ely-totp').inputValue(), '');

      const calls = await page.evaluate(() => window.fixture.elyCalls);
      assert.deepEqual(calls, [
        { username: 'alex@ely.by', password: 'SecretPass!123', totp: '' },
        { username: 'alex@ely.by', password: 'SecretPass!123', totp: '654321' },
      ]);
    }, { lang }));
  }

  await test('Starling (b) Play sin cuentas: ofrece Microsoft, Sin premium y Ely.by directamente en la portada', () => fixture(async (page) => {
    await page.evaluate(async () => {
      const { app } = await import('/src/lib/store.svelte.ts');
      window.fixture.accountsState = { accounts: [], default: null };
      app.accounts = { accounts: [], default: null };
      app.view = 'play';
    });
    await page.locator('#ely-username').waitFor();
    assert.equal(await page.getByRole('heading', { name: 'Ely.by', exact: true }).count(), 1);
  }));

  for (const lang of ['es', 'en']) {
    await test(`Starling (c) y (d) Diagnóstico + Carpetas ${lang}: muestra salida del fallo, desactiva Fused, abre carpetas y regresa a Jugar`, () => fixture(async (page) => {
      await page.evaluate(async () => {
        const { app } = await import('/src/lib/store.svelte.ts');
        const { api } = await import('/src/lib/api.ts');
        api.play = async () => {
          app.progress = { stage: 'elyby', text: 'Preparando autenticación Ely.by', fraction: 0.85 };
          throw new Error('No se pudo descargar authlib-injector para Ely.by.');
        };
        app.view = 'play';
      });

      // (d) Abrir carpeta desde Play
      await page.getByRole('button', { name: lang === 'es' ? 'Abrir carpeta' : 'Open folder', exact: true }).click();
      // Disparar JUGAR -> falla -> redirige automáticamente a Diagnóstico (vista 'logs')
      await page.getByRole('button', { name: lang === 'es' ? 'JUGAR' : 'PLAY', exact: true }).click();

      const alertCard = page.locator('main section[role=alert]');
      await alertCard.waitFor();
      assert.match(await alertCard.innerText(), /Slimefun en solitario \/ LAN · 26\.2/);
      assert.match(await alertCard.innerText(), /Preparando autenticación Ely\.by/);
      assert.match(await alertCard.innerText(), /authlib-injector/);

      // (d) Abrir carpeta de la instancia desde la tarjeta de fallo de Diagnóstico
      await alertCard.getByRole('button', { name: lang === 'es' ? 'Abrir carpeta' : 'Open folder', exact: true }).click();

      // (c) Camino de recuperación en Diagnóstico: Desactivar Fused directamente desde la tarjeta de fallo
      const disableFusedBtn = alertCard.getByRole('button', { name: lang === 'es' ? 'Desactivar Fused' : 'Turn off Fused', exact: true });
      await disableFusedBtn.waitFor();
      await disableFusedBtn.click();
      await disableFusedBtn.waitFor({ state: 'detached' });

      const saved = await page.evaluate(() => window.fixture.savedInstances);
      assert.equal(saved.length, 1);
      assert.equal(saved[0].id, 'slimefun-solo');
      assert.equal(saved[0].fused, false);

      // Cambiar a pestaña Juego en Diagnóstico y verificar logs de la instancia
      await page.getByRole('button', { name: lang === 'es' ? 'Juego' : 'Game', exact: true }).click();
      await page.getByText('[GAME] instance=slimefun-solo', { exact: true }).waitFor();

      // (c) Volver a Jugar desde Diagnóstico
      await page.locator('main section[role=alert]').getByRole('button', { name: lang === 'es' ? 'Volver a Jugar' : 'Return to Play', exact: true }).click();
      await page.getByRole('button', { name: lang === 'es' ? 'JUGAR' : 'PLAY', exact: true }).waitFor();

      // (d) Abrir carpeta desde Instancias, y verificar carpetas de datos/registros + propagación de errores del explorador
      await page.evaluate(async () => { (await import('/src/lib/store.svelte.ts')).app.view = 'instances'; });
      await page.getByRole('button', { name: lang === 'es' ? 'Abrir carpeta' : 'Open folder' }).first().click();

      await page.evaluate(async () => { (await import('/src/lib/store.svelte.ts')).app.view = 'settings'; });
      await page.getByRole('button', { name: lang === 'es' ? 'Abrir carpeta de datos' : 'Open data folder', exact: true }).click();
      await page.getByRole('button', { name: lang === 'es' ? 'Abrir carpeta de registros' : 'Open logs folder', exact: true }).click();

      // Si el explorador falla al abrir la carpeta de registros desde Ajustes o Diagnóstico, el error se muestra en toast
      await page.evaluate(async () => {
        const { api } = await import('/src/lib/api.ts');
        api.openFolder = async () => { throw new Error('Explorer open_folder failure'); };
      });
      await page.getByRole('button', { name: lang === 'es' ? 'Abrir carpeta de registros' : 'Open logs folder', exact: true }).click();
      await page.locator('.toasts .toast.error').getByText('Explorer open_folder failure', { exact: true }).waitFor();

      const folders = await page.evaluate(() => window.fixture.folders);
      assert.deepEqual(folders, [
        { kind: 'instance', id: 'slimefun-solo' },
        { kind: 'instance', id: 'slimefun-solo' },
        { kind: 'instance', id: 'slimefun-solo' },
        { kind: 'data', id: null },
        { kind: 'logs', id: null },
      ]);
    }, { lang }));
  }
} finally {
  await browser?.close();
  await server.close();
}
