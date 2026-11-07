// Espejo de los tipos serializados por drakestar-core / drakestar-app.
export type Mode = 'drakes-rust' | 'drakes-java' | 'local-server' | 'online';
export type Channel = 'stable' | 'unstable' | 'nightly';
export type Loader = 'vanilla' | 'fabric' | 'forge' | 'neoforge';
export type Support = 'yes' | 'no' | 'planned';

export interface JavaSettings {
  major: number | null;
  min_mem_mb: number;
  max_mem_mb: number;
  extra_jvm_args: string[];
}
export interface Profile {
  schema: number;
  id: string;
  name: string;
  mode: Mode;
  channel: Channel;
  minecraft: string;
  loader: Loader;
  java: JavaSettings;
  server: string | null;
  preset: string | null;
  modpack: string | null;
  notes: string;
}
export interface ProfileList {
  profiles: Profile[];
  invalid: { id: string; error: string }[];
}
export interface Cell { support: Support; caveat: string | null }
export interface ModeCompat {
  mode: Mode;
  label: string;
  loaders: { loader: Loader; cell: Cell }[];
  allowed_loaders: Loader[];
  modpacks_allowed: boolean;
  server_plugins: Cell;
  slimefun_gui: Cell;
  launch_implemented: boolean;
  launch_note: string;
}
export interface AppInfo {
  name: string;
  version: string;
  data_dir: string;
  platform: string;
  /** 'tauri' = backend Rust real; 'mock' = simulacion en el navegador. */
  backend: 'tauri' | 'mock';
}
export interface Settings { java_path: string | null; active_profile: string | null }
export interface JavaInfo { bin: string; found: boolean; version: string | null; major: number | null; error: string | null }
export interface JavaCheck { info: JavaInfo; warning: string | null }

export type ProcState = 'stopped' | 'running' | 'stopping' | 'crashed';
export interface ProcStatus {
  state: ProcState;
  pid: number | null;
  started_at: number | null;
  exit_code: number | null;
  log_file: string | null;
}
export type StreamKind = 'stdout' | 'stderr' | 'system' | 'stdin';
export interface LineRec { ts: number; stream: StreamKind; line: string }
export type ProcEvent =
  | { type: 'line'; profile: string; ts: number; stream: StreamKind; line: string }
  | { type: 'state'; profile: string; state: ProcState; code: number | null };

export interface InstanceInfo {
  id: string; path: string; data_dir: string; data_exists: boolean; size_bytes: number;
  modified: number | null; server_jar: string | null; launcher_logs: number;
}
export interface InstanceRow { profile: Profile; info: InstanceInfo; status: ProcStatus }
export interface LogFile { source: 'launcher' | 'server'; name: string; size: number; modified: number | null }

export interface ServerStatus {
  dir: string; exists: boolean;
  jar: { selected: string | null; candidates: string[] };
  eula: boolean | null; has_properties: boolean; plugins: number; worlds: number;
}
export interface Prop { key: string; value: string }
export interface PluginInfo { file: string; name: string; enabled: boolean; size: number }
export interface WorldInfo { name: string; size: number; modified: number | null }

export interface Resolution {
  profile: string;
  profile_channel: Channel;
  component: string | null;
  platform: string;
  minecraft: string;
  resolved: null | {
    version: string; release_channel: Channel; url: string; sha256: string; size: number;
    sha256_wellformed: boolean; notes: string;
  };
  reason: string | null;
}
