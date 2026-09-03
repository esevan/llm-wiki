import react from '@vitejs/plugin-react';
import { defineConfig } from 'vitest/config';

export default defineConfig(({ mode }) => ({
  root: 'frontend',
  define: mode === 'test' ? {} : { 'process.env.NODE_ENV': JSON.stringify('production') },
  plugins: [react()],
  build: {
    assetsInlineLimit: 0,
    emptyOutDir: false,
    outDir: '../llm_wiki/static/assets',
    lib: {
      entry: 'src/main.tsx',
      formats: ['iife'],
      name: 'LlmWikiReact',
      fileName: () => 'app.js',
    },
    rollupOptions: {
      output: {
        assetFileNames: (assetInfo) =>
          assetInfo.names.some((name) => name.endsWith('.css'))
            ? 'app.css'
            : 'fonts/[name]-[hash][extname]',
      },
    },
  },
  test: {
    environment: 'jsdom',
    setupFiles: ['src/test/setup.ts'],
  },
}));
