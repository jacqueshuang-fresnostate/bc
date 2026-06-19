import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import tailwindcss from '@tailwindcss/vite'

const env = (globalThis as { process?: { env?: Record<string, string | undefined> } }).process?.env ?? {}
const devApiBase = env.VITE_API_BASE_URL || env.VITE_API_BASE || 'http://127.0.0.1:8080'

export default defineConfig({
  plugins: [vue(), tailwindcss()],
  server: {
    port: 5173,
    proxy: {
      '/api': { target: devApiBase, changeOrigin: true },
      '/ws': { target: devApiBase, ws: true },
    },
  },
})
