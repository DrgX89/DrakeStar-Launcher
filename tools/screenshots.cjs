// Capturas de la UI en MODO NAVEGADOR (backend mock). Uso: vite en :5173 y
//   NODE_PATH=<dir con playwright> node tools/screenshots.cjs <raiz-del-repo>
const { chromium } = require('playwright');
const fs = require('fs');
const root = process.argv[2];
const out = (n) => `${root}/docs/screenshots/${n}.png`;
const assert = (c, m) => { if (!c) { console.error('FALLO:', m); process.exitCode = 1; } else console.log('ok  :', m); };
(async () => {
  const b = await chromium.launch();
  const pg = await b.newPage({ viewport: { width: 1280, height: 800 } });
  const errs = [];
  pg.on('console', (m) => { if (m.type() === 'error') errs.push(m.text()); });
  pg.on('pageerror', (e) => errs.push('pageerror: ' + e.message));
  const go = async (r) => { await pg.evaluate((r) => (location.hash = r), r); await pg.waitForTimeout(350); };
  await pg.goto('http://localhost:5173/');
  await pg.waitForSelector('text=Qué funciona hoy');
  await pg.waitForTimeout(500);
  await pg.screenshot({ path: out('01-inicio') });

  await go('/profiles');
  await pg.screenshot({ path: out('02-perfiles') });

  // editor: modo DrakesRust no admite loaders
  await pg.getByRole('button', { name: 'Nuevo perfil' }).click();
  await pg.getByLabel('Nombre').fill('Rust de prueba');
  await pg.locator('label', { hasText: 'DrakesRust' }).click();
  const fabricOpt = pg.locator('#pf-loader option[value=fabric]');
  const loaderSel = await pg.locator('#pf-loader').count();
  assert(loaderSel === 1, 'selector de loader visible para DrakesRust');
  assert(await fabricOpt.isDisabled(), 'loader fabric deshabilitado para DrakesRust');
  assert((await pg.locator('#pf-pack').isDisabled()), 'modpack deshabilitado para DrakesRust');
  await pg.locator('summary', { hasText: 'Compatibilidades' }).click();
  await pg.waitForTimeout(300);
  await pg.screenshot({ path: out('03-editor-perfil-rust') });
  const txt = await pg.locator('dialog[open]').innerText();
  assert(/sin verificar/i.test(txt), 'DrakesRust muestra "sin verificar"');
  // cambiar a DrakesJava, elegir fabric + modpack, luego volver a Rust -> se limpia
  await pg.locator('label', { hasText: 'DrakesJava' }).click();
  await pg.locator('#pf-loader').selectOption('fabric');
  await pg.locator('#pf-pack').fill('mi-pack');
  await pg.waitForTimeout(400);
  assert(!(await pg.locator('dialog[open] .alert.err').count()), 'java+fabric+modpack valido');
  await pg.locator('label', { hasText: 'DrakesRust' }).click();
  await pg.waitForTimeout(400);
  assert((await pg.locator('#pf-loader').inputValue()) === 'vanilla', 'al pasar a Rust el loader vuelve a vanilla');
  await pg.getByRole('button', { name: 'Crear perfil' }).click();
  await pg.waitForTimeout(500);
  assert(await pg.locator('article', { hasText: 'Rust de prueba' }).count() === 1, 'perfil Rust creado y listado');

  // importacion invalida: rust + forge
  await pg.getByRole('button', { name: 'Importar JSON' }).click();
  await pg.locator('#imp-text').fill('{"schema":1,"id":"mal","name":"Mal","mode":"drakes-rust","channel":"stable","minecraft":"26.2","loader":"forge"}');
  await pg.getByRole('button', { name: 'Importar', exact: true }).click();
  await pg.waitForTimeout(300);
  assert(/no admite loaders/.test(await pg.locator('.toast').innerText()), 'importar rust+forge se rechaza con mensaje');
  await pg.getByRole('button', { name: 'Cerrar' }).first().click();

  // Local Forge: EULA, arrancar, consola
  await pg.locator('article', { hasText: 'Local Dev' }).getByRole('button', { name: 'Activar' }).click().catch(() => {});
  await go('/forge');
  await pg.waitForSelector('text=Carpeta del servidor');
  await pg.getByRole('button', { name: 'Iniciar' }).first().click();
  await pg.waitForTimeout(300);
  assert(/EULA/.test(await pg.locator('.toast').innerText()), 'arrancar sin EULA se rechaza');
  await pg.screenshot({ path: out('04-local-forge') });
  await pg.getByLabel(/He leído y acepto/).check();
  await pg.getByRole('button', { name: 'Escribir eula=true' }).click();
  await pg.waitForTimeout(400);
  await pg.getByRole('button', { name: 'Iniciar' }).first().click();
  await pg.waitForSelector('h1:has-text("Consola")');
  await pg.waitForTimeout(1900);
  await pg.getByLabel('Comando para el servidor').fill('list');
  await pg.getByRole('button', { name: 'Enviar' }).click();
  await pg.waitForTimeout(500);
  const con = await pg.locator('.console').first().innerText();
  assert(/Done \(1\.4s\)/.test(con) && /list/.test(con), 'consola muestra salida y comando');
  await pg.screenshot({ path: out('05-consola') });
  await pg.getByRole('button', { name: 'Detener' }).first().click();
  await pg.waitForTimeout(1200);

  await go('/forge/plugins');
  await pg.screenshot({ path: out('06-plugins') });

  // modo Rust: activar el perfil creado
  await go('/profiles');
  await pg.locator('article', { hasText: 'Rust de prueba' }).getByRole('button', { name: 'Activar' }).click();
  await pg.waitForTimeout(300);
  await go('/rust');
  await pg.screenshot({ path: out('07-drakesrust') });
  assert(/no puede lanzar el cliente/.test(await pg.locator('main').innerText()), 'DrakesRust declara que no puede lanzar');

  // actualizaciones con el manifiesto de ejemplo
  await go('/settings/updates');
  const manifest = fs.readFileSync(`${root}/examples/manifest.example.json`, 'utf8');
  await pg.locator('#mf').fill(manifest);
  await pg.getByRole('button', { name: 'Importar manifiesto' }).click();
  await pg.waitForTimeout(600);
  await pg.screenshot({ path: out('08-actualizaciones') });

  await go('/instances');
  await pg.waitForTimeout(300);
  await pg.screenshot({ path: out('09-instancias') });

  // responsive
  await pg.setViewportSize({ width: 390, height: 844 });
  await go('/profiles');
  await pg.waitForTimeout(300);
  await pg.screenshot({ path: out('10-movil-perfiles') });
  const overflow = await pg.evaluate(() => document.documentElement.scrollWidth > window.innerWidth + 1);
  assert(!overflow, 'sin desbordamiento horizontal en 390px');

  console.log('errores de consola:', JSON.stringify(errs));
  await b.close();
})();
