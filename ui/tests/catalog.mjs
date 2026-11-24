// Chromium + Svelte reales; backend sintetico, sin descargas ni cuentas.
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
  async function fixture(run, { lang = 'en', width = 1280 } = {}) {
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
        app.instances = ['first', 'second'].map((id) => ({
          id, name: id, mc: '26.2', loader: 'fabric', target: { kind: 'menu' },
          performance: false, fused: false, xmx_mb: 2048, extra_jvm: [], note: '',
        }));
        app.selected = 'first';
        window.fixture = { lists: [], changes: [] };
        api.catalogList = (id) => new Promise((resolve, reject) => window.fixture.lists.push({ id, resolve, reject }));
        api.catalogSearch = async () => [{ project_id: 'fixture-project', title: 'Fixture project', description: 'Synthetic content' }];
        api.catalogChange = (id, kind, project, remove) => new Promise((resolve, reject) => window.fixture.changes.push({ id, kind, project, remove, resolve, reject }));
        app.view = 'catalog';
      }, { lang });
      await page.waitForFunction(() => window.fixture.lists.length === 1);
      await run(page);
      assert.deepEqual(errors, []);
    } finally {
      await context.close();
    }
  }
  async function install(page, kind = 'mod', lang = 'en') {
    await page.locator('#catalog-kind').selectOption(kind);
    await page.locator('form button[type=submit]').click();
    await page.getByRole('button', { name: lang === 'es' ? 'Instalar' : 'Install', exact: true }).click();
    await page.waitForFunction(() => window.fixture.changes.length === 1);
    assert.deepEqual(await page.evaluate(() => {
      const { id, kind, project, remove } = window.fixture.changes[0];
      return { id, kind, project, remove };
    }), { id: 'first', kind, project: 'fixture-project', remove: false });
  }
  async function settleList(page, outcome, index = 0) {
    await page.evaluate(({ outcome, index }) => {
      const read = window.fixture.lists[index];
      if (outcome === 'error') read.reject(new Error('Obsolete list error'));
      else read.resolve([]);
    }, { outcome, index });
    // Deja ejecutar tambien los efectos y actualizaciones DOM de Svelte.
    await page.evaluate(() => new Promise(requestAnimationFrame));
  }
  async function settleChange(page, kind, error = false) {
    await page.evaluate(({ kind, error }) => {
      const change = window.fixture.changes[0];
      if (error) change.reject(new Error('Current install error'));
      else change.resolve([{ project_id: 'fixture-project', title: 'Fixture project', kind, filename: 'fixture-content.jar', owned: true }]);
    }, { kind, error });
  }

  for (const kind of ['mod', 'resourcepack', 'shader']) {
    for (const order of ['old-first', 'new-first']) {
      for (const outcome of ['success', 'error']) {
        await test(`catalogo ${kind}/${order}/${outcome}: la lectura inicial no pisa la instalacion`, () => fixture(async (page) => {
          await install(page, kind);
          if (order === 'old-first') await settleList(page, outcome);
          await settleChange(page, kind);
          await page.getByText(`${kind} · fixture-content.jar`, { exact: true }).waitFor();
          if (order === 'new-first') await settleList(page, outcome);
          assert.equal(await page.getByText(`${kind} · fixture-content.jar`, { exact: true }).count(), 1);
          assert.equal(await page.locator('main [role=alert]').count(), 0);
        }));
      }
    }
  }
  for (const outcome of ['success', 'error']) {
    await test(`catalogo: fallo parcial conserva dependencias y diagnostico ante lectura antigua ${outcome}`, () => fixture(async (page) => {
      await install(page);
      await settleChange(page, 'mod', true);
      await page.waitForFunction(() => window.fixture.lists.length === 2);
      await page.evaluate(() => window.fixture.lists[1].resolve([
        { project_id: 'dependency', title: 'Dependency', kind: 'mod', filename: 'dependency.jar', owned: true },
      ]));
      await page.getByText('mod · dependency.jar', { exact: true }).waitFor();
      await settleList(page, outcome);
      assert.equal(await page.getByText('mod · dependency.jar', { exact: true }).count(), 1);
      assert.equal(await page.locator('main [role=alert]').innerText(), 'Current install error');
    }));
  }
  for (const outcome of ['success', 'error']) {
    await test(`catalogo: ${outcome} de otra instancia no contamina la vista vigente`, () => fixture(async (page) => {
      await install(page);
      await page.evaluate(async () => (await import('/src/lib/store.svelte.ts')).app.select('second'));
      await page.waitForFunction(() => window.fixture.lists.length === 2);
      await settleList(page, 'success', 1);
      await settleChange(page, 'mod', outcome === 'error');
      await page.waitForFunction(async () => !(await import('/src/lib/store.svelte.ts')).app.catalogBusy);
      assert.equal(await page.getByText('mod · fixture-content.jar', { exact: true }).count(), 0);
      assert.equal(await page.locator('main [role=alert]').count(), 0);
      assert.equal(await page.evaluate(() => window.fixture.lists.length), 2);
    }));
  }
  for (const outcome of ['success', 'error']) {
    await test(`catalogo: volver durante una instalacion ${outcome} espera el catalogo final`, () => fixture(async (page) => {
      await install(page);
      await page.evaluate(async () => (await import('/src/lib/store.svelte.ts')).app.view = 'logs');
      await page.locator('#catalog-instance').waitFor({ state: 'detached' });
      await page.evaluate(async () => (await import('/src/lib/store.svelte.ts')).app.view = 'catalog');
      await page.locator('#catalog-instance').waitFor();
      assert.equal(await page.evaluate(() => window.fixture.lists.length), 1);
      await settleChange(page, 'mod', outcome === 'error');
      await page.waitForFunction(() => window.fixture.lists.length === 2);
      await page.evaluate(() => window.fixture.lists[1].resolve([
        { project_id: 'dependency', title: 'Dependency', kind: 'mod', filename: 'final-content.jar', owned: true },
      ]));
      await page.getByText('mod · final-content.jar', { exact: true }).waitFor();
      await settleList(page, 'success');
      assert.equal(await page.getByText('mod · final-content.jar', { exact: true }).count(), 1);
      assert.equal(await page.evaluate(() => window.fixture.lists.length), 2);
      assert.equal(await page.locator('main [role=alert]').count(), 0);
    }));
  }
  for (const lang of ['es', 'en']) {
    await test(`catalogo ${lang}/375: instalar, quitar y reintentar conserva la operacion vigente`, () => fixture(async (page) => {
      await install(page, 'resourcepack', lang);
      await settleChange(page, 'resourcepack');
      const installed = page.getByText('resourcepack · fixture-content.jar', { exact: true });
      await installed.waitFor();
      await settleList(page, 'success');
      assert.equal(await installed.count(), 1);
      const item = installed.locator('..').locator('..');
      await item.getByRole('button', { name: lang === 'es' ? 'Quitar' : 'Remove', exact: true }).click();
      await item.getByRole('button', { name: lang === 'es' ? 'Sí' : 'Yes', exact: true }).click();
      await page.waitForFunction(() => window.fixture.changes.length === 2);
      assert.equal(await page.evaluate(() => window.fixture.changes[1].remove), true);
      await page.evaluate(() => window.fixture.changes[1].resolve([]));
      await installed.waitFor({ state: 'detached' });
      await page.getByRole('button', { name: lang === 'es' ? 'Instalar' : 'Install', exact: true }).click();
      await page.waitForFunction(() => window.fixture.changes.length === 3);
      await page.evaluate(() => window.fixture.changes[2].resolve([
        { project_id: 'fixture-project', title: 'Fixture project', kind: 'resourcepack', filename: 'fixture-content.jar', owned: true },
      ]));
      await installed.waitFor();
      assert.equal(await page.locator('main [role=alert]').count(), 0);
    }, { lang, width: 375 }));
  }
} finally {
  await browser?.close();
  await server.close();
}
