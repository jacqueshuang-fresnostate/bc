import type { OutputAsset } from 'rollup'
import { defineConfig, type Plugin } from 'vite'
import vue from '@vitejs/plugin-vue'
import tailwindcss from '@tailwindcss/vite'

const env = (globalThis as { process?: { env?: Record<string, string | undefined> } }).process?.env ?? {}
const devApiBase = env.VITE_API_BASE_URL || env.VITE_API_BASE || 'http://127.0.0.1:8080'

function inlineBuiltCssForTauri(): Plugin {
  return {
    name: 'bc-inline-built-css-for-tauri',
    apply: 'build',
    enforce: 'post',
    generateBundle(_, bundle) {
      const htmlAsset = Object.values(bundle).find((asset): asset is OutputAsset => (
        asset.type === 'asset' && asset.fileName === 'index.html'
      ))
      if (!htmlAsset || typeof htmlAsset.source !== 'string') return

      const cssAssets = Object.values(bundle).filter((asset): asset is OutputAsset => (
        asset.type === 'asset' && asset.fileName.endsWith('.css') && typeof asset.source === 'string'
      ))
      if (!cssAssets.length) return

      const cssText = cssAssets.map(asset => String(asset.source)).join('\n').replace(/<\/style/gi, '<\\/style')
      htmlAsset.source = htmlAsset.source
        .replace(/\s*<link\s+rel="stylesheet"[^>]*href="\.?\/?assets\/[^"]+\.css"[^>]*>\s*/g, '\n')
        .replace('</head>', `    <style data-tauri-inline-css>\n${cssText}\n    </style>\n  </head>`)

      // 部分 Android WebView 对 Tauri 自定义协议下的外链 CSS 会偶发跳过加载，内联后删除冗余 CSS 资源。
      for (const asset of cssAssets) {
        delete bundle[asset.fileName]
      }
    },
  }
}

export default defineConfig({
  base: './',
  plugins: [vue(), tailwindcss(), inlineBuiltCssForTauri()],
  build: {
    // 部分手机 WebView 对异步页面 CSS chunk 加载不稳定，移动端统一打进首屏样式包。
    cssCodeSplit: false,
  },
  server: {
    port: 5173,
    proxy: {
      '/api': { target: devApiBase, changeOrigin: true },
      '/ws': { target: devApiBase, ws: true },
    },
  },
})
