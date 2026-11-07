import type { Channel, Mode, ProcState } from './types';
// Los SVG viven en assets/brand y se copian a public/brand (npm run sync-brand).
const B = './brand';
export const brand = {
  logo: `${B}/logo.svg`,
  isotype: `${B}/isotype.svg`,
  banner: `${B}/banner.svg`,
  channel: (c: Channel) => `${B}/ui/status-channel-${c}.svg`,
  modeIcon: (m: Mode) => `${B}/icons/${{ 'drakes-rust': 'mode-drakesrust', 'drakes-java': 'mode-drakesjava', 'local-server': 'mode-localserver', online: 'mode-online' }[m]}.svg`,
  tab: (m: Mode, active: boolean) => `${B}/ui/tab-${{ 'drakes-rust': 'rust', 'drakes-java': 'java', 'local-server': 'server', online: 'online' }[m]}-${active ? 'active' : 'idle'}.svg`,
  statePill: (s: ProcState) => ({ running: `${B}/ui/status-online.svg`, stopped: `${B}/ui/status-offline.svg`, crashed: `${B}/ui/status-error.svg`, stopping: null })[s],
  experimental: `${B}/ui/status-experimental.svg`,
  icon: (n: 'console' | 'play' | 'plugin' | 'profile' | 'settings' | 'slimefun' | 'stop' | 'update' | 'world' | 'modpack') => `${B}/icons/${n}.svg`,
};
export const modeLabel: Record<Mode, string> = { 'drakes-rust': 'DrakesRust', 'drakes-java': 'DrakesJava', 'local-server': 'Local Forge', online: 'Online' };
export const modeHome: Record<Mode, string> = { 'drakes-rust': '/rust', 'drakes-java': '/java', 'local-server': '/forge', online: '/online' };
export const stateLabel: Record<ProcState, string> = { running: 'En ejecución', stopped: 'Detenido', stopping: 'Deteniendo…', crashed: 'Terminó con error' };
