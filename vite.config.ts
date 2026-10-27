import react from '@vitejs/plugin-react';
import { defineConfig } from 'vitest/config';

export default defineConfig(({ mode }) => ({
  root: 'frontend',
  define: mode === 'test' ? {} : { 'process.env.NODE_ENV': JSON.stringify('production') },
  plugins: [react()],
  build: {
    assetsInlineLimit: 0,
    emptyOutDir: true,
    outDir: '../dist',
    lib: {
      entry: 'src/main.tsx',
      formats: ['iife'],
      name: 'LlmWikiReact',
      fileName: () => 'assets/app.js',
    },
    rollupOptions: {
      output: {
        assetFileNames: (assetInfo) =>
          assetInfo.names.some((name) => name.endsWith('.css'))
            ? 'assets/app.css'
            : 'assets/fonts/[name]-[hash][extname]',
      },
    },
  },
  test: {
    // Preserve only this explicit raw stylesheet import for the reduced-motion
    // CSSOM regression; ordinary component CSS remains stubbed in jsdom.
    css: { include: [/task-sessions\.css\?raw$/] },
    environment: 'jsdom',
    setupFiles: ['src/test/setup.ts'],
  },
}));
