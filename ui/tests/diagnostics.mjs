// Prueba el estado Svelte compilado para cliente, sin red ni procesos de juego.
import assert from 'node:assert/strict';
import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { test } from 'node:test';
import { build } from 'vite';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const out = await mkdtemp(join(tmpdir(), 'drakestar-diagnostics-'));
try {
  const entry = join(out, 'entry.mjs');
  await writeFile(entry,
    `export { app } from ${JSON.stringify(join(root, 'src/lib/store.svelte.ts'))};
     export { api } from ${JSON.stringify(join(root, 'src/lib/api.ts'))};`);
  await build({
    root,
    configFile: join(root, 'vite.config.ts'),
    logLevel: 'error',
    build: {
      outDir: join(out, 'build'),
      emptyOutDir: true,
      minify: false,
      lib: { entry, formats: ['es'], fileName: () => 'state.mjs' },
    },
  });
  const { app, api } = await import(pathToFileURL(join(out, 'build/state.mjs')).href);
  const originalApi = { ...api };
  const first = { id: 'first', name: 'First', mc: '26.2', target: { kind: 'menu' } };
  const second = { id: 'second', name: 'Second', mc: '26.2', target: { kind: 'server', address: 'example.invalid' } };
  const ready = { binaries: true, resources: true, java: true, ready: true, reason: null };
  function reset() {
    Object.assign(api, originalApi);
    Object.assign(app, {
      instances: [first, second], selected: 'second', instancesError: null,
      catalogBusy: false, phase: 'idle', view: 'play', hz: ready,
      attemptInstance: first, failedStage: 'Previous attempt',
      message: 'Previous error', exitCode: 9, progress: null,
      gameLines: ['Previous output'], warnings: ['Previous warning'], report: null,
    });
  }

  await test('Horizon conserva la instancia intentada aunque cambie la seleccion', async () => {
    reset();
    api.horizonPlay = async (id, server) => {
      assert.equal(id, 'second');
      assert.equal(server, 'example.invalid');
      assert.equal(app.failedStage, '');
      assert.deepEqual([...app.gameLines], []);
      app.select('first');
      throw new Error('Synthetic Horizon failure');
    };
    await app.playHorizon();
    assert.equal(app.phase, 'error');
    assert.equal(app.view, 'logs');
    assert.equal(app.attemptInstance.id, 'second');
    assert.equal(app.failedStage, '');
    assert.equal(app.message, 'Synthetic Horizon failure');
    assert.equal(app.exitCode, null);
    let reported;
    api.report = async (id, send) => { reported = id; return { path: 'synthetic.zip', sent: send }; };
    app.toast = () => {}; // Sin temporizadores de notificaciones en las pruebas.
    await app.makeReport(false);
    assert.equal(reported, 'second');
    app.instances.find((i) => i.id === 'second').name = 'Edited after failure';
    assert.equal(app.attemptInstance.name, 'Second');
  });

  await test('fallo al consultar Horizon abre diagnostico para el intento actual', async () => {
    reset();
    app.hz = null;
    api.horizonStatus = async () => { throw new Error('Synthetic status failure'); };
    await app.playHorizon();
    assert.equal(app.view, 'logs');
    assert.equal(app.attemptInstance.id, 'second');
    assert.equal(app.message, 'Synthetic status failure');
  });

  await test('fallo de instalacion Horizon conserva su etapa en diagnostico', async () => {
    reset();
    app.hz = { ...ready, binaries: false };
    api.horizonInstall = async () => {
      app.progress = { stage: 'horizon', text: 'Synthetic install stage', fraction: 0.25 };
      throw new Error('Synthetic install failure');
    };
    await app.playHorizon();
    assert.equal(app.view, 'logs');
    assert.equal(app.attemptInstance.id, 'second');
    assert.equal(app.failedStage, 'Synthetic install stage');
    assert.equal(app.progress, null);
    assert.equal(app.message, 'Synthetic install failure');
  });

  for (const method of ['play', 'playHorizon']) {
    await test(`${method}: preserva la etapa del fallo y permite volver a intentar`, async () => {
      reset();
      const apiMethod = method === 'play' ? 'play' : 'horizonPlay';
      api[apiMethod] = async () => {
        app.progress = { stage: 'assets', text: 'Synthetic assets stage', fraction: 0.5 };
        throw new Error('Synthetic preparation failure');
      };
      await app[method]();
      assert.equal(app.failedStage, 'Synthetic assets stage');
      assert.equal(app.progress, null);
      assert.equal(app.view, 'logs');
      assert.equal(app.busy, false);
      app.select('first');
      api[apiMethod] = async () => {};
      await app[method]();
      assert.equal(app.attemptInstance.id, 'first');
      assert.equal(app.failedStage, '');
      assert.equal(app.message, '');
    });

    await test(`${method}: no reemplaza el intento mientras el juego esta ocupado`, async () => {
      reset();
      app.phase = 'running';
      const previous = app.attemptInstance;
      api.play = api.horizonPlay = async () => assert.fail('No debe lanzar otra partida');
      await app[method]();
      assert.equal(app.attemptInstance, previous);
      assert.equal(app.phase, 'running');
    });
  }

  await test('desactivar Fused recupera la instancia fallida y conserva sus ediciones recientes', async () => {
    reset();
    app.phase = 'error';
    app.attemptInstance = { ...first, fused: true, name: 'Old name' };
    app.instances = [{ ...first, fused: true, name: 'Updated name' }, { ...second, fused: false }];
    app.toast = () => {};
    let saved;
    api.saveInstance = async (instance) => { saved = instance; return instance; };
    api.instances = async () => [saved, { ...second, fused: false }];
    assert.equal(app.recoveryInstance.id, 'first');
    assert.equal(app.recoveryInstance.fused, true);
    await app.disableFused();
    assert.equal(saved.id, 'first');
    assert.equal(saved.fused, false);
    assert.equal(saved.name, 'Updated name');
    assert.equal(app.instances.find((i) => i.id === 'second').fused, false);
    assert.equal(app.attemptInstance.fused, true);
    assert.equal(app.recoveryInstance.fused, false);
  });

  for (const unavailable of ['deleted', 'unreadable']) {
    await test(`recuperacion con catalogo ${unavailable} no modifica otra instancia`, async () => {
      reset();
      app.phase = 'error';
      app.instances = [{ ...second, fused: true }];
      if (unavailable === 'unreadable') app.instancesError = 'Synthetic catalog error';
      api.saveInstance = async () => assert.fail('No debe guardar otra instancia');
      assert.equal(app.recoveryInstance, null);
      await app.disableFused();
      assert.equal(app.instances[0].fused, true);
    });
  }

  await test('sin intento previo la recuperacion conserva la seleccion actual', () => {
    reset();
    app.attemptInstance = null;
    assert.equal(app.recoveryInstance.id, 'second');
  });

  for (const method of ['play', 'playHorizon']) {
    for (const outcome of ['success', 'error']) {
      await test(`${method}: ignora reporte ${outcome} del intento anterior de la misma instancia`, async () => {
        reset();
        app.selected = 'first';
        const toasts = [];
        app.toast = (...args) => toasts.push(args);
        let complete;
        api.report = (id) => {
          assert.equal(id, 'first');
          return new Promise((resolve, reject) => {
            complete = () => outcome === 'success'
              ? resolve({ path: 'previous.zip', sent: true })
              : reject(new Error('Previous report failure'));
          });
        };
        const pending = app.makeReport(true);
        api[method === 'play' ? 'play' : 'horizonPlay'] = async () => {
          throw new Error('Current launch failure');
        };
        await app[method]();
        assert.equal(app.attemptInstance.id, 'first');
        complete();
        await pending;
        assert.equal(app.report, null);
        assert.deepEqual(toasts, []);
        assert.equal(app.message, 'Current launch failure');
      });
    }
  }

  for (const order of ['old-first', 'new-first']) {
    await test(`reportes concurrentes ${order}: solo la ultima solicitud actualiza el diagnostico`, async () => {
      reset();
      const toasts = [];
      app.toast = (...args) => toasts.push(args);
      const finish = [];
      api.report = async () => new Promise((resolve) => finish.push(resolve));
      const old = app.makeReport(false);
      const current = app.makeReport(true);
      const currentReport = { path: 'current.zip', sent: true };
      if (order === 'old-first') {
        finish[0]({ path: 'previous.zip', sent: false });
        await old;
        assert.equal(app.report, null);
      }
      finish[1](currentReport);
      await current;
      if (order === 'new-first') {
        finish[0]({ path: 'previous.zip', sent: false });
        await old;
      }
      assert.deepEqual(app.report, currentReport);
      assert.equal(toasts.length, 1);
      assert.equal(toasts[0][0], 'ok');
    });
  }

  for (const outcome of ['success', 'error']) {
    await test(`el reporte vigente conserva su resultado ${outcome}`, async () => {
      reset();
      const toasts = [];
      app.toast = (...args) => toasts.push(args);
      api.report = async () => {
        if (outcome === 'error') throw new Error('Current report failure');
        return { path: 'current.zip', sent: false };
      };
      await app.makeReport(false);
      assert.equal(toasts.length, 1);
      assert.equal(toasts[0][0], outcome === 'success' ? 'ok' : 'error');
      if (outcome === 'success') assert.equal(app.report.path, 'current.zip');
      else assert.equal(toasts[0][1], 'Current report failure');
    });
  }
} finally {
  await rm(out, { recursive: true, force: true });
}
