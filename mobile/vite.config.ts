import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import tailwindcss from '@tailwindcss/vite'

export default defineConfig({
  plugins: [vue(), tailwindcss()],
  server: {
    port: 5173,
    proxy: {
      '/api': { target: 'https://ad.16888888.live', changeOrigin: true },
      '/ws': { target: 'https://ad.16888888.live', ws: true },
      // '/api': { target: 'http://127.0.0.1:18120', changeOrigin: true },
      // '/ws': { target: 'http://127.0.0.1:18120', ws: true },
    },
  },
})
