// Copia los SVG de marca (fuente unica: assets/brand, generados por tools/gen_brand.py)
// a ui/public/brand para que Vite los sirva/empaquete. No se redibuja nada.
import { cpSync, rmSync, existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const here = dirname(fileURLToPath(import.meta.url));
const src = join(here, '..', '..', 'assets', 'brand');
const dst = join(here, '..', 'public', 'brand');
if (!existsSync(src)) throw new Error(`no existe ${src}`);
rmSync(dst, { recursive: true, force: true });
cpSync(src, dst, { recursive: true });
console.log(`marca sincronizada: ${src} -> ${dst}`);
