import type { OutputAsset } from 'rollup'
import { defineConfig, type Plugin } from 'vite'
import vue from '@vitejs/plugin-vue'
import tailwindcss from '@tailwindcss/vite'

const env = (globalThis as { process?: { env?: Record<string, string | undefined> } }).process?.env ?? {}
const devApiBase = env.VITE_API_BASE_URL || env.VITE_API_BASE || 'http://127.0.0.1:8080'

function isAtRuleBoundary(source: string, index: number): boolean {
  const next = source[index]
  return !next || /\s|;|\{|\(/.test(next)
}

function findAtRuleHeaderEnd(source: string, start: number): number {
  let quote: string | null = null
  for (let index = start; index < source.length; index += 1) {
    const char = source[index]
    const next = source[index + 1]

    if (quote) {
      if (char === '\\') {
        index += 1
      } else if (char === quote) {
        quote = null
      }
      continue
    }

    if (char === '"' || char === "'") {
      quote = char
      continue
    }
    if (char === '/' && next === '*') {
      const commentEnd = source.indexOf('*/', index + 2)
      index = commentEnd >= 0 ? commentEnd + 1 : source.length
      continue
    }
    if (char === ';' || char === '{') {
      return index
    }
  }
  return source.length
}

function findMatchingBrace(source: string, openIndex: number): number {
  let depth = 0
  let quote: string | null = null

  for (let index = openIndex; index < source.length; index += 1) {
    const char = source[index]
    const next = source[index + 1]

    if (quote) {
      if (char === '\\') {
        index += 1
      } else if (char === quote) {
        quote = null
      }
      continue
    }

    if (char === '"' || char === "'") {
      quote = char
      continue
    }
    if (char === '/' && next === '*') {
      const commentEnd = source.indexOf('*/', index + 2)
      index = commentEnd >= 0 ? commentEnd + 1 : source.length
      continue
    }
    if (char === '{') {
      depth += 1
    } else if (char === '}') {
      depth -= 1
      if (depth === 0) {
        return index
      }
    }
  }

  return source.length - 1
}

function rewriteCssAtRule(
  css: string,
  name: string,
  handlers: {
    block: (header: string, inner: string) => string
    statement: (statement: string) => string
  },
): string {
  const token = `@${name}`
  let output = ''
  let cursor = 0

  while (cursor < css.length) {
    const atIndex = css.indexOf(token, cursor)
    if (atIndex < 0) {
      output += css.slice(cursor)
      break
    }
    const tokenEnd = atIndex + token.length
    if (!isAtRuleBoundary(css, tokenEnd)) {
      output += css.slice(cursor, tokenEnd)
      cursor = tokenEnd
      continue
    }

    output += css.slice(cursor, atIndex)
    const headerEnd = findAtRuleHeaderEnd(css, tokenEnd)
    if (headerEnd >= css.length || css[headerEnd] === ';') {
      output += handlers.statement(css.slice(atIndex, Math.min(headerEnd + 1, css.length)))
      cursor = Math.min(headerEnd + 1, css.length)
      continue
    }

    const closeIndex = findMatchingBrace(css, headerEnd)
    const header = css.slice(atIndex, headerEnd).trim()
    const inner = css.slice(headerEnd + 1, closeIndex)
    output += handlers.block(header, inner)
    cursor = Math.min(closeIndex + 1, css.length)
  }

  return output
}

function flattenCssLayerRules(css: string): string {
  let previous = css
  while (true) {
    const next = rewriteCssAtRule(previous, 'layer', {
      block: (_, inner) => inner,
      statement: () => '',
    })
    if (next === previous) {
      return next
    }
    previous = next
  }
}

function removeCssPropertyRules(css: string): string {
  return rewriteCssAtRule(css, 'property', {
    block: () => '',
    statement: () => '',
  })
}

function removeModernColorSupports(css: string): string {
  return rewriteCssAtRule(css, 'supports', {
    block: (header, inner) => (
      /color-mix|oklch/i.test(header) ? '' : `${header}{${removeModernColorSupports(inner)}}`
    ),
    statement: statement => statement,
  })
}

function clampColorChannel(value: number): number {
  return Math.max(0, Math.min(255, Math.round(value)))
}

function linearSrgbToDisplay(value: number): number {
  const clamped = Math.max(0, Math.min(1, value))
  return clamped <= 0.0031308
    ? clamped * 12.92
    : 1.055 * Math.pow(clamped, 1 / 2.4) - 0.055
}

function oklchToRgb(luminance: number, chroma: number, hue: number): [number, number, number] {
  const radians = (hue * Math.PI) / 180
  const a = chroma * Math.cos(radians)
  const b = chroma * Math.sin(radians)
  const lPrime = luminance + 0.3963377774 * a + 0.2158037573 * b
  const mPrime = luminance - 0.1055613458 * a - 0.0638541728 * b
  const sPrime = luminance - 0.0894841775 * a - 1.291485548 * b
  const l = lPrime ** 3
  const m = mPrime ** 3
  const s = sPrime ** 3

  const red = 4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s
  const green = -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s
  const blue = -0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s

  return [
    clampColorChannel(linearSrgbToDisplay(red) * 255),
    clampColorChannel(linearSrgbToDisplay(green) * 255),
    clampColorChannel(linearSrgbToDisplay(blue) * 255),
  ]
}

function normalizeOklchToken(match: string, lToken: string, cToken: string, hToken: string, alphaToken?: string): string {
  const luminance = lToken.endsWith('%')
    ? Number.parseFloat(lToken) / 100
    : Number.parseFloat(lToken)
  const chroma = Number.parseFloat(cToken)
  const hue = Number.parseFloat(hToken)
  if (![luminance, chroma, hue].every(Number.isFinite)) {
    return match
  }

  const [red, green, blue] = oklchToRgb(luminance, chroma, hue)
  if (!alphaToken) {
    return `rgb(${red}, ${green}, ${blue})`
  }
  const alpha = alphaToken.endsWith('%')
    ? Number.parseFloat(alphaToken) / 100
    : Number.parseFloat(alphaToken)
  if (!Number.isFinite(alpha) || alpha >= 1) {
    return `rgb(${red}, ${green}, ${blue})`
  }
  return `rgba(${red}, ${green}, ${blue}, ${Math.max(0, Math.min(1, alpha))})`
}

function replaceOklchColors(css: string): string {
  return css.replace(
    /oklch\(\s*([0-9.]+%?)\s+([0-9.]+)\s+([0-9.]+)(?:deg)?(?:\s*\/\s*([0-9.]+%?))?\s*\)/gi,
    normalizeOklchToken,
  )
}

function normalizeBuiltCssForLegacyAndroidWebView(css: string): string {
  return replaceOklchColors(removeModernColorSupports(removeCssPropertyRules(flattenCssLayerRules(css))))
}

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

      const cssText = normalizeBuiltCssForLegacyAndroidWebView(cssAssets.map(asset => String(asset.source)).join('\n'))
        .replace(/<\/style/gi, '<\\/style')
      htmlAsset.source = htmlAsset.source
        .replace(/\s*<link\s+rel="stylesheet"[^>]*href="\.?\/?assets\/[^"]+\.css"[^>]*>\s*/g, '\n')
        .replace('</head>', `    <style data-tauri-inline-css>\n${cssText}\n    </style>\n  </head>`)

      // 部分旧 Android WebView 不支持 Tailwind 4 输出的 @layer/@property/oklch，先降级再内联样式。
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

    // proxy: {
    //   '/api': { target: 'http://127.0.0.1:18120', changeOrigin: true },
    //   '/ws': { target: 'http://127.0.0.1:18120', ws: true },
    // },
  },
})
