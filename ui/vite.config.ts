import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { spawn } from 'child_process';

function drakesLaunchPlugin() {
  return {
    name: 'drakes-launcher-backend',
    configureServer(server: any) {
      server.middlewares.use('/api/launch-rust', (req: any, res: any) => {
        try {
          const batPath = 'C:\\Users\\Jack\\DrakesRust\\connect-drakescraft.bat';
          const p = spawn('cmd.exe', ['/c', batPath], {
            detached: true,
            stdio: 'ignore',
            cwd: 'C:\\Users\\Jack\\DrakesRust\\bin'
          });
          p.unref();
          res.setHeader('Content-Type', 'application/json');
          res.end(JSON.stringify({ ok: true, message: 'DrakesRust Client lanzado' }));
        } catch (err: any) {
          res.statusCode = 500;
          res.end(JSON.stringify({ ok: false, error: err.message }));
        }
      });
      server.middlewares.use('/api/launch-horizon', (req: any, res: any) => {
        try {
          const exePath = 'C:\\Users\\Jack\\DrakesRust\\bin\\drakesrust-launcher.exe';
          const p = spawn(exePath, [], {
            detached: true,
            stdio: 'ignore',
            cwd: 'C:\\Users\\Jack\\DrakesRust\\bin'
          });
          p.unref();
          res.setHeader('Content-Type', 'application/json');
          res.end(JSON.stringify({ ok: true, message: 'Launcher DrakesRust abierto' }));
        } catch (err: any) {
          res.statusCode = 500;
          res.end(JSON.stringify({ ok: false, error: err.message }));
        }
      });
    }
  };
}

export default defineConfig({
  plugins: [svelte(), drakesLaunchPlugin()],
  base: './',
  clearScreen: false,
  resolve: { preserveSymlinks: true },
  server: {
    port: 5173,
    strictPort: true,
    host: true,
    proxy: {
      '/api/jarvis': {
        target: 'http://100.114.161.74:8096',
        changeOrigin: true
      },
      '/jarvis.html': {
        target: 'http://100.114.161.74:8096',
        changeOrigin: true
      }
    }
  },
  build: { target: 'es2022', sourcemap: false },
});
