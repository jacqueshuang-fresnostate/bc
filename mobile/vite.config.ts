import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import tailwindcss from '@tailwindcss/vite'

export default defineConfig({
  plugins: [vue(), tailwindcss()],
  server: {
    port: 5173,
    proxy: {
      // '/api': { target: 'https://bc.hippo-web3.cc.cd', changeOrigin: true },
      // '/ws': { target: 'https://bc.hippo-web3.cc.cd', ws: true },
      '/api': { target: 'http://127.0.0.1:18120', changeOrigin: true },
      '/ws': { target: 'http://127.0.0.1:18120', ws: true },
    },
  },
})
