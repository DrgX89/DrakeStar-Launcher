// ============================ MOCK (SOLO MODO NAVEGADOR) ============================
// Simula los comandos Tauri en memoria para desarrollar/capturar la UI sin el backend.
// NO ejecuta procesos, NO escribe en disco, NO verifica nada. Las reglas de validacion
// y la matriz de compatibilidad estan duplicadas aqui a mano y pueden divergir de
// drakestar-core: la fuente de verdad es Rust. La UI muestra un aviso permanente.
import type { Profile, ProfileList, ModeCompat, Loader, Mode, ProcEvent, ProcStatus, LineRec, StreamKind, Settings, Prop, PluginInfo, Resolution, InstanceRow, LogFile } from './types';

const listeners = new Set<(e: ProcEvent) => void>();
export async function mockListen(cb: (e: ProcEvent) => void) {
  listeners.add(cb);
  return () => { listeners.delete(cb); };
}
const emit = (e: ProcEvent) => listeners.forEach((l) => l(e));

const base = (o: Partial<Profile> & Pick<Profile, 'id' | 'name' | 'mode'>): Profile => ({
  schema: 1, channel: 'stable', minecraft: '26.2', loader: 'vanilla',
  java: { major: null, min_mem_mb: 1024, max_mem_mb: 4096, extra_jvm_args: [] },
  server: null, preset: null, modpack: null, notes: '', ...o,
});
let profiles: Profile[] = [
  base({ id: 'local-dev', name: 'Local Dev (Slimefun)', mode: 'local-server', channel: 'unstable', preset: 'slimefun', java: { major: 25, min_mem_mb: 2048, max_mem_mb: 6144, extra_jvm_args: [] }, notes: 'Servidor de pruebas DrakesPurpur con el preset Slimefun.' }),
  base({ id: 'rust-nightly', name: 'DrakesRust (nightly)', mode: 'drakes-rust', channel: 'nightly' }),
  base({ id: 'drakescraft', name: 'DrakesCraft', mode: 'online', server: 'play.drakescraft.example' }),
  base({ id: 'java-fabric', name: 'Java Fabric 26.2', mode: 'drakes-java', loader: 'fabric' }),
];
let settings: Settings = { java_path: null, active_profile: 'local-dev' };
let manifestReleases: any[] | null = null;
let eula = false;
let jarPresent = true;
const props: Prop[] = [
  { key: 'motd', value: 'DrakeStar Dev' }, { key: 'server-port', value: '25565' }, { key: 'max-players', value: '8' },
  { key: 'online-mode', value: 'false' }, { key: 'difficulty', value: 'normal' }, { key: 'gamemode', value: 'survival' },
  { key: 'view-distance', value: '10' }, { key: 'white-list', value: 'false' }, { key: 'level-name', value: 'world' },
];
let plugins: PluginInfo[] = [
  { file: 'Slimefun.jar', name: 'Slimefun', enabled: true, size: 4_812_330 },
  { file: 'StarSuites.jar', name: 'StarSuites', enabled: true, size: 1_204_100 },
  { file: 'Dynmap.jar.disabled', name: 'Dynmap', enabled: false, size: 3_100_000 },
];

const COMPAT: ModeCompat[] = (() => {
  const loaders = (mode: Mode) => (['vanilla', 'fabric', 'forge', 'neoforge'] as Loader[]).map((l) => ({
    loader: l,
    cell: l === 'vanilla' ? { support: 'yes' as const, caveat: null }
      : mode === 'drakes-java' ? { support: 'planned' as const, caveat: 'previsto, no implementado' }
      : { support: 'no' as const, caveat: mode === 'drakes-rust' ? 'otro runtime: un cliente Rust no carga mods Java' : null },
  }));
  const mk = (mode: Mode, label: string, launch: boolean, note: string, plug: ModeCompat['server_plugins'], sf: ModeCompat['slimefun_gui']): ModeCompat => {
    const ls = loaders(mode);
    return { mode, label, loaders: ls, allowed_loaders: ls.filter((x) => x.cell.support !== 'no').map((x) => x.loader), modpacks_allowed: mode === 'drakes-java', server_plugins: plug, slimefun_gui: sf, launch_implemented: launch, launch_note: note };
  };
  const yes = { support: 'yes' as const, caveat: null };
  return [
    mk('drakes-rust', 'DrakesRust', false, 'no existe cliente Rust empaquetado: el perfil se gestiona, pero no se puede lanzar',
      { support: 'yes', caveat: 'los plugins viven en el servidor; que un cliente Rust conecte a DrakesPurpur 26.2 NO esta verificado' },
      { support: 'planned', caveat: 'sin verificar: requiere protocolo de inventarios completo en el cliente' }),
    mk('drakes-java', 'DrakesJava', false, 'lanzamiento de cliente Java pendiente (JRE gestionado, loaders y modpacks: v0.4)', yes, yes),
    mk('local-server', 'Local Forge', true, 'arranca el JAR con el plan de drakestar-core (requiere Java y un JAR en la carpeta del servidor)', yes,
      { support: 'no', caveat: 'no aplica: depende del cliente con el que te conectes' }),
    mk('online', 'Online', false, 'lanzar el cliente depende de DrakesJava/DrakesRust; aqui solo hay perfiles y ping TCP', yes,
      { support: 'no', caveat: 'no aplica: depende del cliente con el que te conectes' }),
  ];
})();

// Equivalente a los #[serde(default)] de drakestar-core al leer JSON parcial.
function withDefaults(p: any): Profile {
  return { schema: 1, channel: 'stable', loader: 'vanilla', server: null, preset: null, modpack: null, notes: '', ...p,
    java: { major: null, min_mem_mb: 1024, max_mem_mb: 4096, extra_jvm_args: [], ...(p?.java ?? {}) } };
}
function validate(p: Profile): string | null {
  if (p.schema !== 1) return `schema ${p.schema} no soportado (esperado 1)`;
  if (!/^[a-z0-9_-]{1,48}$/.test(p.id)) return `id invalido '${p.id}' (usa a-z, 0-9, '-' y '_')`;
  if (p.java.min_mem_mb > p.java.max_mem_mb) return 'min_mem_mb no puede superar max_mem_mb';
  switch (p.mode) {
    case 'drakes-rust':
      if (p.loader !== 'vanilla') return 'drakes-rust no admite loaders Java (fabric/forge/neoforge)';
      if (p.modpack) return 'drakes-rust no admite modpacks Java';
      break;
    case 'drakes-java':
      if (p.preset) return "'preset' solo aplica a local-server";
      if (p.modpack && p.loader === 'vanilla') return 'un modpack requiere loader fabric/forge/neoforge';
      break;
    case 'local-server':
      if (p.loader !== 'vanilla' || p.modpack) return 'local-server (Purpur/Paper) no usa loaders ni modpacks de cliente';
      break;
    case 'online':
      if (p.loader !== 'vanilla' || p.modpack) return 'online es un perfil de conexion: sin loader ni modpack';
      if (!p.server) return "online requiere 'server'";
      break;
  }
  return null;
}

// ---- procesos simulados ----
const status: Record<string, ProcStatus> = {};
const hist: Record<string, LineRec[]> = {};
const timers: Record<string, number[]> = {};
function line(id: string, stream: StreamKind, text: string) {
  const rec = { ts: Date.now(), stream, line: text };
  (hist[id] ??= []).push(rec);
  emit({ type: 'line', profile: id, ...rec });
}
function setState(id: string, state: ProcStatus['state'], code: number | null = null) {
  status[id] = { ...(status[id] ?? { pid: null, started_at: null, exit_code: null, log_file: null }), state, exit_code: code ?? status[id]?.exit_code ?? null };
  emit({ type: 'state', profile: id, state, code });
}
const stopped = (): ProcStatus => ({ state: 'stopped', pid: null, started_at: null, exit_code: null, log_file: null });

function need<T>(v: T | undefined, what: string): T { if (v === undefined) throw `no encontrado: ${what}`; return v; }
function prof(id: string) { return need(profiles.find((p) => p.id === id), `perfil '${id}'`); }
const delay = <T,>(v: T, ms = 40) => new Promise<T>((r) => setTimeout(() => r(v), ms));

export async function mockInvoke(cmd: string, a: Record<string, any> = {}): Promise<any> {
  switch (cmd) {
    case 'app_info': return { name: 'DrakeStar', version: '0.1.0', data_dir: '(mock: sin disco)', platform: 'navegador', backend: 'mock' };
    case 'compat_report': return COMPAT;
    case 'list_profiles': return { profiles: [...profiles].sort((x, y) => x.name.localeCompare(y.name)), invalid: [] } satisfies ProfileList;
    case 'validate_profile': return validate(a.profile);
    case 'create_profile': {
      const e = validate(a.profile); if (e) throw e;
      if (profiles.some((p) => p.id === a.profile.id)) throw `ya existe: perfil '${a.profile.id}'`;
      profiles.push(a.profile); return a.profile;
    }
    case 'update_profile': {
      const e = validate(a.profile); if (e) throw e;
      prof(a.profile.id); profiles = profiles.map((p) => (p.id === a.profile.id ? a.profile : p)); return a.profile;
    }
    case 'delete_profile': prof(a.id); profiles = profiles.filter((p) => p.id !== a.id); if (settings.active_profile === a.id) settings.active_profile = null; return;
    case 'duplicate_profile': {
      const p = { ...structuredClone(prof(a.id)), id: a.newId, name: a.newName };
      const e = validate(p); if (e) throw e;
      if (profiles.some((x) => x.id === p.id)) throw `ya existe: perfil '${p.id}'`;
      profiles.push(p); return p;
    }
    case 'import_profile': {
      let p: Profile; try { p = withDefaults(JSON.parse(a.json)); } catch (e) { throw `JSON invalido: ${e}`; }
      const e = validate(p); if (e) throw e;
      const exists = profiles.some((x) => x.id === p.id);
      if (exists && !a.overwrite) throw `ya existe: perfil '${p.id}'`;
      profiles = exists ? profiles.map((x) => (x.id === p.id ? p : x)) : [...profiles, p]; return p;
    }
    case 'export_profile': return JSON.stringify(prof(a.id), null, 2);
    case 'export_profile_file': prof(a.id); return `(mock) exports/${a.id}.json`;
    case 'get_settings': return settings;
    case 'set_settings': settings = a.settings; return settings;
    case 'java_check': return delay({ info: { bin: settings.java_path ?? 'java', found: true, version: '25.0.4 (simulado)', major: 25, error: null }, warning: null });
    case 'list_instances': return profiles.map((p): InstanceRow => ({
      profile: p, status: status[p.id] ?? stopped(),
      info: { id: p.id, path: `(mock)/profiles/${p.id}`, data_dir: `(mock)/profiles/${p.id}/${p.mode === 'local-server' ? 'server' : 'instance'}`, data_exists: p.mode !== 'online', size_bytes: p.mode === 'local-server' ? 183_402_113 : p.mode === 'online' ? 0 : 4096, modified: Date.now() - 3600_000, server_jar: p.mode === 'local-server' && jarPresent ? 'drakespurpur.jar' : null, launcher_logs: p.mode === 'local-server' ? 2 : 0 },
    }));
    case 'ensure_instance': return;
    case 'list_logs': return [{ source: 'launcher', name: 'launcher-1730000000000.log', size: 2210, modified: Date.now() - 7200_000 }, { source: 'server', name: 'latest.log', size: 8120, modified: Date.now() - 600_000 }] satisfies LogFile[];
    case 'read_log': return ['(mock) contenido simulado del log', `${a.source}/${a.name}`];
    case 'clear_logs': return 1;
    case 'server_plan': return 'java -Xms2048M -Xmx6144M --enable-native-access=ALL-UNNAMED -jar drakespurpur.jar --nogui\n(cwd: (mock)/profiles/' + a.id + '/server)';
    case 'server_start': {
      prof(a.id);
      if (!jarPresent) throw 'no encontrado: no hay un unico JAR de servidor (coloca drakespurpur.jar o instala una release)';
      if (!eula) throw 'EULA de Minecraft sin aceptar: aceptala desde la pagina Local Forge antes de arrancar';
      const id = a.id as string;
      if (status[id] && ['running', 'stopping'].includes(status[id].state)) throw `'${id}' ya esta en ejecucion`;
      hist[id] = hist[id] ?? [];
      status[id] = { state: 'running', pid: 4242, started_at: Date.now(), exit_code: null, log_file: '(mock)' };
      emit({ type: 'state', profile: id, state: 'running', code: null });
      line(id, 'system', '--- [MOCK] inicio simulado: no se ejecuta ningun proceso ---');
      const script: [number, StreamKind, string][] = [
        [300, 'stdout', '[Server thread/INFO]: Starting minecraft server version 26.2'],
        [700, 'stdout', '[Server thread/INFO]: Loading properties'],
        [1100, 'stderr', '[Server thread/WARN]: (simulado) aviso de ejemplo en stderr'],
        [1500, 'stdout', '[Server thread/INFO]: Done (1.4s)! For help, type "help"'],
      ];
      timers[id] = script.map(([ms, s, t]) => window.setTimeout(() => line(id, s, t), ms));
      return status[id];
    }
    case 'server_command': {
      const st = status[a.id]; if (!st || st.state !== 'running') throw `'${a.id}' no esta en ejecucion`;
      line(a.id, 'stdin', a.line);
      window.setTimeout(() => line(a.id, 'stdout', `[Server thread/INFO]: (simulado) comando recibido: ${a.line}`), 120);
      return;
    }
    case 'server_stop': {
      const st = status[a.id]; if (!st || st.state !== 'running') throw `'${a.id}' no esta en ejecucion`;
      timers[a.id]?.forEach(clearTimeout); setState(a.id, 'stopping');
      line(a.id, 'stdin', 'stop');
      window.setTimeout(() => { line(a.id, 'stdout', '[Server thread/INFO]: Stopping server'); line(a.id, 'system', '--- [MOCK] proceso terminado (codigo 0) ---'); setState(a.id, 'stopped', 0); }, 700);
      return;
    }
    case 'server_kill': timers[a.id]?.forEach(clearTimeout); line(a.id, 'system', '--- [MOCK] terminado a la fuerza ---'); setState(a.id, 'stopped', null); return;
    case 'proc_status': return status[a.id] ?? stopped();
    case 'console_history': return hist[a.id] ?? [];
    case 'console_clear': hist[a.id] = []; return;
    case 'server_status': prof(a.id); return { dir: `(mock)/profiles/${a.id}/server`, exists: true, jar: { selected: jarPresent ? 'drakespurpur.jar' : null, candidates: jarPresent ? ['drakespurpur.jar'] : [] }, eula: eula ? true : false, has_properties: true, plugins: plugins.filter((p) => p.enabled).length, worlds: 2 };
    case 'server_accept_eula': eula = true; return;
    case 'server_properties': return props;
    case 'server_set_properties': for (const [k, v] of Object.entries(a.updates as Record<string, string>)) { const e = props.find((p) => p.key === k); if (e) e.value = v; else props.push({ key: k, value: v }); } return props;
    case 'server_plugins': return plugins;
    case 'server_toggle_plugin': plugins = plugins.map((p) => (p.file === a.file ? { ...p, enabled: !p.enabled, file: p.enabled ? `${p.name}.jar.disabled` : `${p.name}.jar` } : p)); return a.file;
    case 'server_worlds': return [{ name: 'world', size: 96_300_221, modified: Date.now() - 86400_000 }, { name: 'world_nether', size: 12_400_000, modified: Date.now() - 86400_000 }];
    case 'manifest_import': {
      let m: any; try { m = JSON.parse(a.json); } catch (e) { throw `JSON invalido: ${e}`; }
      if (m.schema !== 1) throw `schema de manifiesto ${m.schema} no soportado (esperado 1)`;
      manifestReleases = m.releases; return m.releases.length;
    }
    case 'manifest_summary': return manifestReleases ? manifestReleases.length : null;
    case 'resolve_release': {
      const p = prof(a.id);
      const comp = p.mode === 'local-server' ? 'drakespurpur' : p.mode === 'drakes-rust' ? 'drakes-rust-client' : null;
      const r: Resolution = { profile: p.id, profile_channel: p.channel, component: comp, platform: 'linux-x86_64', minecraft: p.minecraft, resolved: null, reason: null };
      if (!comp) { r.reason = 'este modo no tiene un componente descargable gestionado todavia'; return r; }
      if (!manifestReleases) { r.reason = 'no hay manifiesto de releases importado'; return r; }
      const order = { stable: 0, unstable: 1, nightly: 2 } as const;
      const cands = manifestReleases.filter((x) => order[x.channel as keyof typeof order] <= order[p.channel] && x.minecraft.includes(p.minecraft))
        .map((x) => ({ x, art: x.artifacts.find((y: any) => y.component === comp) })).filter((c) => c.art);
      cands.sort((l, r2) => String(l.x.version).localeCompare(String(r2.x.version), undefined, { numeric: true }));
      const best = cands.at(-1);
      if (!best) { r.reason = `ninguna release de '${comp}' para linux-x86_64 y Minecraft ${p.minecraft} es aceptable en el canal ${p.channel}`; return r; }
      r.resolved = { version: best.x.version, release_channel: best.x.channel, url: best.art.url, sha256: best.art.sha256, size: best.art.size, sha256_wellformed: /^[0-9a-fA-F]{64}$/.test(best.art.sha256), notes: best.x.notes ?? '' };
      return r;
    }
    case 'install_server_jar': throw '[MOCK] la descarga real solo existe en la app Tauri';
    case 'ping_server': throw '[MOCK] el ping TCP real solo existe en la app Tauri';
    case 'mods_report':
    case 'mods_decide': return delay({ discovered: 138, verified: 138, order: ['fabric-api', 'sodium', 'distanthorizons'], not_loaded: [], abi: [['fabric-api', 'Java (JVM)', '2.0'], ['sodium', 'Java (JVM)', '2.0']], classpath: [], warnings: [], bytes: 157_000_000, skipped: ['algo-forge.jar: no es Fabric'] });
    case 'rust_client_status': return { profile: a.id, binary: '(mock)', installed: false, executable: false, version: null, ready: false, reason: 'el cliente no esta instalado todavia' };
    case 'rust_client_plan': throw 'no encontrado: cliente no instalado';
    case 'rust_client_install': throw '[MOCK] la descarga real solo existe en la app Tauri';
    case 'rust_client_start': throw 'no encontrado: cliente no instalado';
    case 'rust_client_stop':
    case 'rust_client_command': throw `'${a.id}' no esta en ejecucion`;
    case 'diagnostics': return JSON.stringify({ app: 'DrakeStar', backend: 'mock', nota: 'diagnostico simulado, no del sistema real' }, null, 2);
    default: throw `comando desconocido en el mock: ${cmd}`;
  }
}
