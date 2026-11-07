# El intermediario (ABI estable entre mods y motor)

## La idea

El loader y el cliente estan escritos en Rust. Los modos pueden ser **Java**
(classpath + JVM) o **Rust nativos** (cdylib). Lo que unifica a los dos es el
**intermediario**: una capa que vive en el **cargador**, no en el cliente.

```
   mods Java ──┐                        ┌───────────────────────────────┐
               ├──►  INTERMEDIARIO      │  · fabric.mod.json (estandar) │
   mods Rust ──┘    (drakestar-core)    │  · ABI estable + hooks        │
                                         │  · compatibilidad por version │
                                         └──────────────┬────────────────┘
                                                        │ tabla de hooks
                                         ┌──────────────▼────────────────┐
                                         │ MOTOR (Rust puro)             │
                                         │ no sabe que existen los mods  │
                                         └───────────────────────────────┘
```

El motor publica una **tabla de hooks**. El intermediario decide, para cada mod,
que entrypoints llamar y en que hooks enganchan. Cuando el motor cambia, se
cambia el adaptador: **los modos no se recompilan**.

## Que es cada pieza

| Pieza | Donde | Que hace |
|---|---|---|
| `mods::ModManifest` | `drakestar-core::mods` | valida `fabric.mod.json` **sin modificarlo** |
| `mods::resolve` | idem | dependencias (dialecto semver de Fabric), conflictos, orden de carga |
| `inter::Catalog` | `drakestar-core::inter` | los hooks que existen y desde que ABI |
| `inter::plan` | idem | el plan de carga de un mod: runtime, ABI, entrypoints, hooks |
| `inter::ABI` | idem | la version del ABI que expone el cargador |

## Reglas de compatibilidad (las que hay que respetar siempre)

1. **Mismo `major`, siempre compatible.** `major` cambia solo si se rompe un
   contrato.
2. **Un mod compilado contra un ABI mas nuevo que el del motor no carga**
   (`Compat::ModNewer`): puede usar hooks que el motor no tiene.
3. **Un mod mas viejo que el motor carga** (`Compat::ModOlder`): todo lo que usa
   existe. Se avisa igual.
4. **Agregar hooks no rompe**; por eso el catalogo lleva `since` por hook.
   Quitarlos si rompe, y por eso existe `removed_in`: se puede deprecar.

## Como declara un mod nativo

El mismo `fabric.mod.json` de siempre, con los metadatos extra en `custom`
(donde el formato ya lo permite). Un mod **Java** no declara ABI: su contrato es
el classpath, y se verifica por manifiesto (entrypoints, hooks, dependencias).

```json
{
  "schemaVersion": 1,
  "id": "mi-addon",
  "version": "0.1.0",
  "environment": "client",
  "entrypoints": { "main": ["drakestar_mod_entry"] },
  "depends": { "minecraft": "^26.2" },
  "custom": {
    "drakes": { "abi": "1", "hooks": { "main": ["mod:init", "client:tick"] } }
  }
}
```

`environment` es `*` (universal), `client` o `server`. Un mod universal puede
enganchar hooks de los dos lados; uno `client` no puede enganchar hooks de
servidor.

## Verlo sin compilar nada

```bash
cargo run -q -p drakestar-cli -- mod-check examples/mod-java.fabric.mod.json
cargo run -q -p drakestar-cli -- mod-check examples/mod-nativo.fabric.mod.json
```

Salida de un mod Java real:

```
OK  slimefun-drakes v2.1.0 [*] entorno=*
    runtime Java (JVM) · ABI 1.0 (compatible)
    entrypoint main     → net.drakes.slimefun.SlimefunMod   hooks: mod:init
    entrypoint server   → net.drakes.slimefun.ServerHooks   hooks: server:init, server:command, server:player_join
    depende de: fabricloader >=0.15, java >=25, minecraft ^26.2
```

Ese comando no necesita red ni JVM: es la primera prueba para saber si un mod
que consigas va a cargar y por que.

## Resolver una carpeta de mods

```bash
cargo run -q -p drakestar-cli -- mods-check ~/.minecraft/mods
# el entorno se puede forzar (para probar otra version):
DS_MC=26.3 cargo run -q -p drakestar-cli -- mods-check ~/.minecraft/mods
```

Lee los `fabric.mod.json` de cada jar **y de sus jar-in-jar** (los 95 modulos
de `fabric-api` viajan asi), resuelve el grafo con `mods::resolve` y muestra el
orden de carga por lado (cliente/servidor) y por que no carga lo que no carga.

## Lo que se aprendio con mods reales (Sodium, DistantHorizons, fabric-api)

| Hallazgo | Correccion |
|---|---|
| `"minecraft": ["26.2.0"]` (lista, no texto) | el manifiesto acepta lista: es un OR, como en Fabric |
| `fabric-api` no tiene ningun entrypoint | un mod sin entrypoints es valido: son los modulos de API |
| `entrypoints.preLaunch` y `modmenu` | se aceptan; cada uno infiere su hook (`preLaunch`→`mod:load`) |
| 95 jar-in-jar dentro de `fabric-api` | se leen como mods, o nada cargaria |

## El cargador (`drakestar-app::mods`)

Ya no solo valida: **decide que se carga**. `Service::client_start` lo corre
antes de arrancar:

```
descubrir  ->  jars de mods/ + sus jar-in-jar (recursivo)
verificar  ->  SHA-256 de cada uno contra mods.lock.json
validar    ->  inter::plan (ABI, hooks, lado)
resolver   ->  mods::resolve (dependencias, entorno, conflictos, orden)
aplicar    ->  lo que no carga queda como <archivo>.jar.disabled (reversible)
```

Puntos que importan:

- **Verificacion real**: el lock se crea la primera vez y despues un mod que
  cambio deja de cargar. Hoy ningun launcher hace eso.
- **Re-habilitacion**: si arreglas la dependencia que faltaba, el cargador
  vuelve a habilitar el mod en la siguiente ejecucion (no hay que renombrar a
  mano).
- **Quien ejecuta sigue siendo Fabric Loader** mientras el adaptador nativo no
  exista; lo que cambio es *quien decide*.

### Probarlo

```bash
# el cargador sobre una carpeta de mods, sin arrancar el juego
DS_MODS=~/Descargas/mods \
  cargo test -q -p drakestar-app mods::tests::real_load -- --ignored --nocapture

# el juego con los mods encima
DRAKESTAR_RUN_DIR=/tmp/opencode/drakestar-run \
  cargo run -q -p drakestar-app --example run_with_mods -- ~/Descargas/mods 26.2
```

Resultado real con esos mods: **138 descubiertos, 138 verificados, 48 en orden
de carga, 0 fuera** — que es exactamente el conjunto (menos los pseudo-mods
`minecraft`/`java`/`fabricloader`) que cargo Fabric en la ejecucion real.

## Adaptador nativo (cdylib contra el ABI C)

Los mods nativos viajan como jar (igual que los de Java) con su
`fabric.mod.json` y el binario dentro. El cargador:

1. lo descubre y verifica por SHA-256 como a cualquier mod;
2. extrae el binario a `instance/natives-mods/<id>/` (no queda suelto en `mods/`);
3. **verifica la version del ABI C antes de llamar una sola funcion**;
4. llama `init(table)` con la tabla de hooks del motor, y despues `tick`/`shutdown`.

El contrato esta en `drakestar-core::native` (`#[repr(C)]`, sin dependencias):
si ese archivo cambia de forma incompatible, el major sube y los mods viejos
dejan de cargar. El mod de prueba (`crates/drakestar-native-example`) **a
proposito no linkea nada nuestro**: solo replica las structs del ABI, que es la
prueba de que el contrato es C puro.

```bash
cargo build -p drakestar-native-example        # deja el .so
cargo test -p drakestar-app native::           # carga, init, tick, shutdown reales
cargo test -p drakestar-app native_mods_are    # el ciclo completo por Service
```

La verificacion de punta a punta: el mod corre 3 ticks de verdad (su contador
interno lo confirma), recibe la tabla del motor, y responde a `shutdown`.

## El motor de demostración

Ya se puede **ver un mod nativo corriendo** sin esperar al cliente Rust:

```bash
cargo build -p drakestar-native-example
cargo run -q -p drakestar-app --example native_motor -- /tmp/opencode/native-mods 3
```

Salida real:

```
cargador: 1 descubiertos, 1 verificados, 1 cargan
adaptador nativo: 1 mod(s) cargados: ["ejemplo"]
--- ejemplo (ABI 1.0) ---
  init ok
  tick 1
  tick 2
  tick 3
  shutdown
--- lo que dijeron los mods (1 linea(s)) ---
  [INF] mod nativo de prueba: init ok
```

El motor publica una `HookTable` de verdad: el mod la recibe en `init`, verifica
su version, y **loguea a traves de ella**. Un motor real (el cliente Rust)
reemplaza `find`/`log` por hooks del juego; el contrato no cambia.

## Pendiente

- El **cargador** (`drakestar-app::mods`): descubrir modos en `mods/`, verificar
  SHA-256 de cada uno, aplicar `mods::resolve` + `inter::plan`, y ejecutar.
- **Adaptador nativo**: cargar el cdylib contra su ABI (la tabla de hooks en C).
- **Adaptador Java**: classpath armado por Rust; Mixin sigue via JVM.
- **Catalogo**: los hooks de arriba son la semilla del ABI 1.0. Crecer por
  versiones menores, nunca quitando sin deprecar.
