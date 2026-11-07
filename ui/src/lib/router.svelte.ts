import routesFile from '../../routes.json';
import type { Mode } from './types';

export interface RouteDef { path: string; title: string; desc: string; mode?: Mode }
export const routes: RouteDef[] = routesFile.routes as RouteDef[];

function current(): string {
  const h = typeof location !== 'undefined' ? location.hash.replace(/^#/, '') : '';
  return h && routes.some((r) => r.path === h) ? h : '/';
}

class Router {
  path = $state(current());
  constructor() {
    if (typeof window !== 'undefined') window.addEventListener('hashchange', () => (this.path = current()));
  }
  get def(): RouteDef {
    return routes.find((r) => r.path === this.path) ?? routes[0];
  }
  go(path: string) {
    location.hash = path;
  }
}
export const router = new Router();
