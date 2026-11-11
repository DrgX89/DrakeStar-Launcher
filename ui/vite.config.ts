import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
  plugins: [svelte()],
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
