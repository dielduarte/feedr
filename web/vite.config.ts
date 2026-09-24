import path from 'node:path'
import tailwindcss from '@tailwindcss/vite'
import react from '@vitejs/plugin-react'
import { defineConfig } from 'vitest/config'

export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: { alias: { '@': path.resolve(import.meta.dirname, 'src') } },
  server: {
    // `feedrsauros serve` provides the API; the dev server only serves the UI.
    proxy: { '/api': 'http://127.0.0.1:7777' },
  },
  test: { environment: 'jsdom' },
})
