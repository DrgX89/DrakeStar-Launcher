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
      msa: { open: false, step: 'starting', code: null, message: '' },
    });
  }

  const settings = {
    language: 'es', azure_client_id: null, send_reports: true, report_endpoint: null,
    device_id: 'synthetic-device', last_instance: null, auto_update: true, first_run_done: true,
  };
  const flush = async () => { for (let i = 0; i < 12; i++) await Promise.resolve(); };
  for (const firstOutcome of ['success', 'error']) {
    for (const secondOutcome of ['success', 'error']) {
      await test(`ajustes concurrentes ${firstOutcome}/${secondOutcome}: conserva privacidad y permite recuperarse`, async () => {
        reset();
        app.settings = { ...settings };
        const toasts = [];
        app.toast = (...args) => toasts.push(args);
        const writes = [];
        let persisted = { ...settings };
        api.settingsSet = (next) => new Promise((resolve, reject) => {
          writes.push({ next: { ...next }, resolve, reject });
        });
        const privacy = app.saveSettings({ send_reports: false });
        const updates = app.saveSettings({ auto_update: false }, false);
        await flush();
        const concurrent = writes.length;
        const complete = (index, outcome) => {
          if (outcome === 'error') writes[index].reject(new Error(`Settings failure ${index}`));
          else {
            persisted = { ...writes[index].next };
            writes[index].resolve(persisted);
          }
        };
        complete(0, firstOutcome);
        await privacy;
        await flush();
        assert.equal(writes.length, 2);
        complete(1, secondOutcome);
        await updates;
        const expected = {
          ...settings, send_reports: firstOutcome === 'error', auto_update: secondOutcome === 'error',
        };
        assert.deepEqual(app.settings, expected);
        assert.deepEqual(persisted, expected);
        assert.equal(concurrent, 1, 'solo una escritura puede estar en curso');
        assert.deepEqual(toasts.filter(([kind]) => kind === 'error').map(([, text]) => text),
          [firstOutcome, secondOutcome].flatMap((outcome, index) => outcome === 'error' ? [`Settings failure ${index}`] : []));
        assert.equal(toasts.filter(([kind]) => kind === 'ok').length, secondOutcome === 'success' ? 1 : 0);
        api.settingsSet = async (next) => next;
        await app.saveSettings({ language: 'es' });
        assert.deepEqual(app.settings, expected);
      });
    }
  }
  await test('ajustes: cada cambio usa la respuesta guardada y captura el parche al solicitarlo', async () => {
    reset();
    app.settings = { ...settings };
    app.toast = () => {};
    const writes = [];
    api.settingsSet = (next) => new Promise((resolve) => writes.push({ next: { ...next }, resolve }));
    const first = app.saveSettings({ report_endpoint: 'https://example.invalid/reports' });
    const patch = { send_reports: false };
    const second = app.saveSettings(patch);
    patch.send_reports = true;
    await flush();
    writes[0].resolve({ ...writes[0].next, device_id: 'backend-device' });
    await first;
    await flush();
    writes[1].resolve(writes[1].next);
    await second;
    assert.equal(app.settings.send_reports, false);
    assert.equal(app.settings.report_endpoint, 'https://example.invalid/reports');
    assert.equal(app.settings.device_id, 'backend-device');
  });
  await test('ajustes: cambios repetidos del mismo campo terminan con el ultimo valor solicitado', async () => {
    reset();
    app.settings = { ...settings };
    app.toast = () => {};
    const writes = [];
    api.settingsSet = async (next) => { writes.push({ ...next }); return next; };
    await Promise.all([
      app.saveSettings({ send_reports: false }), app.saveSettings({ send_reports: true }),
      app.saveSettings({ send_reports: false }),
    ]);
    assert.deepEqual(writes.map((next) => next.send_reports), [false, true, false]);
    assert.equal(app.settings.send_reports, false);
  });
  await test('ajustes: fallo al refrescar Microsoft no pierde el guardado ni bloquea el cambio siguiente', async () => {
    reset();
    app.settings = { ...settings };
    const toasts = [];
    app.toast = (...args) => toasts.push(args);
    api.settingsSet = async (next) => next;
    api.appInfo = async () => { throw new Error('App metadata unavailable'); };
    await Promise.all([
      app.saveSettings({ azure_client_id: 'synthetic-client' }),
      app.saveSettings({ send_reports: false }),
    ]);
    assert.equal(app.settings.azure_client_id, 'synthetic-client');
    assert.equal(app.settings.send_reports, false);
    assert.deepEqual(toasts, [['error', 'App metadata unavailable']]);
  });
  await test('ajustes: sin configuracion cargada no escribe datos', async () => {
    reset();
    app.settings = null;
    api.settingsSet = async () => assert.fail('No debe crear una configuracion incompleta');
    await app.saveSettings({ send_reports: false });
    assert.equal(app.settings, null);
  });

  const originalAccount = { id: 'original', kind: 'offline', name: 'Original', uuid: 'synthetic-original', added_at: 1 };
  const currentAccount = { ...originalAccount, id: 'current', name: 'Current', uuid: 'synthetic-current' };
  const deviceCode = (user_code) => ({ user_code, verification_uri: 'https://example.invalid', expires_in: 600, interval: 5 });
  for (const stage of ['cancel', 'start']) {
    await test(`Microsoft: error vigente durante ${stage} sigue visible y permite reintentar`, async () => {
      reset();
      let fail = true;
      api.msaCancel = async () => { if (fail && stage === 'cancel') throw new Error('Current authentication error'); };
      api.msaStart = async () => {
        if (fail) throw new Error('Current authentication error');
        return deviceCode('RETRY');
      };
      await app.startMsa();
      assert.equal(app.msa.open, true);
      assert.equal(app.msa.step, 'error');
      assert.equal(app.msa.message, 'Current authentication error');
      fail = false;
      await app.startMsa();
      assert.equal(app.msa.open, true);
      assert.equal(app.msa.step, 'waiting');
      assert.equal(app.msa.code.user_code, 'RETRY');
      assert.equal(app.msa.message, '');
    });
  }
  for (const stage of ['cancel', 'start']) {
    for (const outcome of ['success', 'error']) {
      await test(`Microsoft: cancelar durante ${stage}/${outcome} mantiene cerrado el dialogo`, async () => {
        reset();
        let finish;
        let starts = 0;
        const pending = new Promise((resolve, reject) => { finish = { resolve, reject }; });
        let cancels = 0;
        api.msaCancel = () => ++cancels === 1 && stage === 'cancel' ? pending : Promise.resolve();
        api.msaStart = () => { starts++; return stage === 'start' ? pending : Promise.resolve(deviceCode('OLD')); };
        const attempt = app.startMsa();
        await Promise.resolve();
        if (stage === 'start') assert.equal(starts, 1);
        await app.cancelMsa();
        const closed = { ...app.msa };
        if (outcome === 'success') finish.resolve(deviceCode('OLD'));
        else finish.reject(new Error('Obsolete device code error'));
        await attempt;
        assert.deepEqual(app.msa, closed);
        assert.equal(app.msa.open, false);
        assert.equal(starts, stage === 'cancel' ? 0 : 1);
      });
    }
  }
  for (const order of ['old-first', 'new-first']) {
    for (const outcome of ['success', 'error']) {
      await test(`Microsoft: intento anterior ${order}/${outcome} no pisa el codigo vigente`, async () => {
        reset();
        const finish = [];
        api.msaCancel = async () => {};
        api.msaStart = () => new Promise((resolve, reject) => finish.push({ resolve, reject }));
        const old = app.startMsa();
        await Promise.resolve();
        const current = app.startMsa();
        await Promise.resolve();
        assert.equal(finish.length, 2);
        const settleOld = () => outcome === 'success'
          ? finish[0].resolve(deviceCode('OLD'))
          : finish[0].reject(new Error('Obsolete device code error'));
        if (order === 'old-first') {
          settleOld();
          await old;
          assert.equal(app.msa.step, 'starting');
          assert.equal(app.msa.code, null);
        }
        finish[1].resolve(deviceCode('CURRENT'));
        await current;
        if (order === 'new-first') { settleOld(); await old; }
        assert.equal(app.msa.open, true);
        assert.equal(app.msa.step, 'waiting');
        assert.equal(app.msa.code.user_code, 'CURRENT');
        assert.equal(app.msa.message, '');
      });
    }
  }
  for (const outcome of ['success', 'error']) {
    await test(`Microsoft: el refresco de cuentas ${outcome} termina con diagnostico`, async () => {
      reset();
      app.accounts = { accounts: [originalAccount], default: originalAccount.id };
      app.msa.open = true;
      const toasts = [];
      app.toast = (...args) => toasts.push(args);
      api.accounts = async () => {
        if (outcome === 'error') throw new Error('Account refresh unavailable');
        return { accounts: [currentAccount], default: currentAccount.id };
      };
      app.onMsa({ phase: 'success', message: 'Session authenticated' });
      await new Promise((resolve) => setImmediate(resolve));
      assert.equal(app.msa.open, false);
      assert.equal(app.account.id, outcome === 'success' ? 'current' : 'original');
      assert.deepEqual(toasts, outcome === 'success'
        ? [['ok', 'Session authenticated']]
        : [['ok', 'Session authenticated'], ['error', 'Account refresh unavailable']]);
    });
  }

  await test('Microsoft: el evento de una ventana cerrada no refresca cuentas', async () => {
    reset();
    let reads = 0;
    const toasts = [];
    app.toast = (...args) => toasts.push(args);
    api.accounts = async () => { reads++; return { accounts: [], default: null }; };
    app.onMsa({ phase: 'success', message: 'Ignored authentication event' });
    await new Promise((resolve) => setImmediate(resolve));
    assert.equal(reads, 0);
    assert.deepEqual(toasts, []);
  });

  for (const order of ['old-first', 'new-first']) {
    for (const previousOutcome of ['success', 'error']) {
      for (const currentOutcome of ['success', 'error']) {
        await test(`cuentas concurrentes ${order}/${previousOutcome}/${currentOutcome}: conserva la ultima lectura`, async () => {
          reset();
          app.accounts = { accounts: [originalAccount], default: originalAccount.id };
          const finish = [];
          api.accounts = () => new Promise((resolve, reject) => finish.push({ resolve, reject }));
          let oldSettled = false;
          const result = (promise) => promise.then(() => null, (e) => e.message);
          const old = result(app.loadAccounts()).then((value) => { oldSettled = true; return value; });
          const current = result(app.loadAccounts());
          const complete = (index, outcome) => outcome === 'success'
            ? finish[index].resolve({ accounts: [index === 0 ? originalAccount : currentAccount], default: index === 0 ? originalAccount.id : currentAccount.id })
            : finish[index].reject(new Error(index === 0 ? 'Obsolete account error' : 'Current account error'));
          let settledBeforeCurrent = false;
          if (order === 'old-first') {
            complete(0, previousOutcome);
            for (let i = 0; i < 5; i++) await Promise.resolve();
            settledBeforeCurrent = oldSettled;
          }
          complete(1, currentOutcome);
          const currentResult = await current;
          if (order === 'new-first') complete(0, previousOutcome);
          const oldResult = await old;
          assert.equal(oldResult, currentResult);
          if (order === 'old-first') assert.equal(settledBeforeCurrent, false);
          assert.equal(currentResult, currentOutcome === 'success' ? null : 'Current account error');
          assert.equal(app.account.id, currentOutcome === 'success' ? 'current' : 'original');
        });
      }
    }
  }

  for (const operation of ['remove', 'select']) {
    await test(`cuentas: ${operation} no se revierte por una lista anterior`, async () => {
      reset();
      app.accounts = { accounts: [originalAccount, currentAccount], default: originalAccount.id };
      const finish = [];
      api.accounts = () => new Promise((resolve, reject) => finish.push({ resolve, reject }));
      const old = app.loadAccounts();
      api.removeAccount = async (id) => assert.equal(id, originalAccount.id);
      api.setDefaultAccount = async (id) => assert.equal(id, currentAccount.id);
      const mutation = operation === 'remove' ? app.removeAccount(originalAccount.id) : app.useAccount(currentAccount.id);
      await Promise.resolve();
      finish[1].resolve({ accounts: operation === 'remove' ? [currentAccount] : [originalAccount, currentAccount], default: currentAccount.id });
      await mutation;
      finish[0].resolve({ accounts: [originalAccount, currentAccount], default: originalAccount.id });
      await old;
      assert.equal(app.account.id, currentAccount.id);
      assert.equal(app.accounts.accounts.length, operation === 'remove' ? 1 : 2);
    });
  }

  await test('cuentas: error vigente se propaga y permite reintentar sin perder la lista anterior', async () => {
    reset();
    app.accounts = { accounts: [originalAccount], default: originalAccount.id };
    api.accounts = async () => { throw new Error('Current account error'); };
    await assert.rejects(app.loadAccounts(), /Current account error/);
    assert.equal(app.account.id, originalAccount.id);
    api.accounts = async () => ({ accounts: [currentAccount], default: currentAccount.id });
    await app.loadAccounts();
    assert.equal(app.account.id, currentAccount.id);
  });

  for (const order of ['old-first', 'new-first']) {
    for (const previousOutcome of ['success', 'error']) {
      for (const currentOutcome of ['success', 'error']) {
        await test(`catalogo concurrente ${order}/${previousOutcome}/${currentOutcome}: conserva la ultima lectura`, async () => {
          reset();
          const finish = [];
          api.instances = () => new Promise((resolve, reject) => finish.push({ resolve, reject }));
          let oldSettled = false;
          const old = app.loadInstances().then((result) => { oldSettled = true; return result; });
          const current = app.loadInstances();
          const complete = (index, outcome) => outcome === 'success'
            ? finish[index].resolve([{ ...first, name: index === 0 ? 'Obsolete profile' : 'Current profile' }])
            : finish[index].reject(new Error(index === 0 ? 'Obsolete read error' : 'Current read error'));
          if (order === 'old-first') {
            complete(0, previousOutcome);
            await Promise.resolve();
            await Promise.resolve();
            assert.equal(app.instancesLoading, true);
            assert.equal(app.instancesError, null);
            assert.equal(app.instances.length, 2);
            assert.equal(oldSettled, false);
          }
          complete(1, currentOutcome);
          const currentResult = await current;
          if (order === 'new-first') complete(0, previousOutcome);
          assert.equal(await old, currentResult);
          assert.equal(currentResult, currentOutcome === 'success');
          assert.equal(app.instancesLoading, false);
          if (currentOutcome === 'success') {
            assert.equal(app.instancesError, null);
            assert.equal(app.instances[0].name, 'Current profile');
            assert.equal(app.selected, 'second');
          } else {
            assert.equal(app.instancesError, 'Current read error');
            assert.equal(app.instances.length, 0);
            assert.equal(app.selected, null);
          }
        });
      }
    }
  }

  await test('guardar espera el resultado de la lectura vigente si su refresco fue reemplazado', async () => {
    reset();
    const saved = { ...first, id: 'saved', name: 'Saved profile' };
    api.saveInstance = async () => saved;
    const finish = [];
    api.instances = () => new Promise((resolve, reject) => finish.push({ resolve, reject }));
    let saveSettled = false;
    const saving = app.saveInstance(saved).then((result) => { saveSettled = true; return result; });
    await Promise.resolve();
    assert.equal(finish.length, 1);
    const refresh = app.loadInstances();
    finish[0].reject(new Error('Obsolete refresh error'));
    await Promise.resolve();
    await Promise.resolve();
    assert.equal(saveSettled, false);
    assert.equal(app.instancesError, null);
    finish[1].resolve([saved]);
    assert.equal(await refresh, true);
    assert.equal(await saving, null);
    assert.equal(app.selected, 'saved');
    assert.equal(app.recoveryInstance, null);
    assert.equal(app.instance.name, 'Saved profile');
  });

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
