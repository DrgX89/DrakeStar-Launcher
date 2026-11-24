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
} finally {
  await rm(out, { recursive: true, force: true });
}
