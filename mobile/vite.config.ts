import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import tailwindcss from '@tailwindcss/vite'

export default defineConfig({
  plugins: [vue(), tailwindcss()],
  server: {
    port: 5173,
    proxy: {
      '/api': { target: 'https://ad.1666666.site', changeOrigin: true },
      '/ws': { target: 'https://ad.1666666.site', ws: true },
    },
  },
})
