#!/usr/bin/env node

import { execFileSync } from 'node:child_process'
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'

const DEFAULT_API_BASE = 'https://ad.16888888.live'

function usage() {
  console.log(`用途：从后台手机端站点配置同步打包用品牌资源。

用法：
  node scripts/sync-branding-assets.mjs --dist-dir dist
  node scripts/sync-branding-assets.mjs --ios-appicon-dir src-tauri/gen/apple/Assets.xcassets/AppIcon.appiconset

参数：
  --api-base <地址>          后端 API 根地址，默认 ${DEFAULT_API_BASE}
  --dist-dir <目录>          写入前端打包资源目录，会生成 app-logo.png、logo.svg、mobile-branding.json
  --ios-appicon-dir <目录>   写入 iOS AppIcon.appiconset 图标目录
  --work-dir <目录>          临时工作目录，默认系统临时目录
  -h, --help                显示帮助
`)
}

function fail(message) {
  console.error(`品牌资源同步失败：${message}`)
  process.exit(1)
}

function normalizeApiBase(value) {
  return String(value || '').trim().replace(/\/+$/, '')
}

function parseArgs(argv) {
  const options = {
    apiBase: process.env.VITE_API_BASE_URL || process.env.VITE_API_BASE || DEFAULT_API_BASE,
    distDir: '',
    iosAppIconDir: '',
    workDir: '',
  }

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index]
    if (arg === '-h' || arg === '--help') {
      usage()
      process.exit(0)
    }
    if (arg === '--api-base') {
      options.apiBase = argv[++index] || ''
      continue
    }
    if (arg === '--dist-dir') {
      options.distDir = argv[++index] || ''
      continue
    }
    if (arg === '--ios-appicon-dir') {
      options.iosAppIconDir = argv[++index] || ''
      continue
    }
    if (arg === '--work-dir') {
      options.workDir = argv[++index] || ''
      continue
    }
    fail(`未知参数：${arg}`)
  }

  options.apiBase = normalizeApiBase(options.apiBase)
  if (!options.apiBase) fail('缺少 --api-base')
  if (!options.distDir && !options.iosAppIconDir) {
    fail('至少需要指定 --dist-dir 或 --ios-appicon-dir')
  }
  return options
}

function ensureDirectory(dir) {
  fs.mkdirSync(dir, { recursive: true })
}

function run(command, args) {
  execFileSync(command, args, { stdio: 'pipe' })
}

function runOutput(command, args) {
  return execFileSync(command, args, { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] })
}

async function fetchJson(url) {
  const response = await fetch(url, { headers: { accept: 'application/json' } })
  if (!response.ok) throw new Error(`请求失败 ${response.status} ${response.statusText}`)
  return response.json()
}

async function downloadFile(url, outputPath) {
  const response = await fetch(url)
  if (!response.ok) throw new Error(`图片下载失败 ${response.status} ${response.statusText}`)
  fs.writeFileSync(outputPath, Buffer.from(await response.arrayBuffer()))
}

function cleanText(value) {
  const text = String(value ?? '').trim()
  return text && text !== '未配置' ? text : ''
}

function normalizeConfig(raw) {
  const data = raw && raw.data ? raw.data : raw
  return {
    platformName: cleanText(data?.platformName || data?.site_name),
    logoImageUrl: cleanText(data?.logoImageUrl || data?.logo_url),
    intro: cleanText(data?.intro || data?.slogan),
  }
}

function convertToPng(sourcePath, outputPath) {
  run('sips', ['-s', 'format', 'png', sourcePath, '--out', outputPath])
}

function readImageSize(sourcePath) {
  const output = runOutput('sips', ['-g', 'pixelWidth', '-g', 'pixelHeight', sourcePath])
  const width = Number.parseInt(output.match(/pixelWidth:\s*(\d+)/)?.[1] || '', 10)
  const height = Number.parseInt(output.match(/pixelHeight:\s*(\d+)/)?.[1] || '', 10)
  if (!Number.isFinite(width) || !Number.isFinite(height) || width <= 0 || height <= 0) {
    fail(`无法读取图片尺寸：${sourcePath}`)
  }
  return { width, height }
}

function makeSquarePng(sourcePath, outputPath, size) {
  const { width, height } = readImageSize(sourcePath)
  const squareSide = Math.min(width, height)
  const croppedPath = `${outputPath}.cropped.png`
  const needsCrop = width !== height
  const inputPath = needsCrop ? croppedPath : sourcePath

  if (needsCrop) {
    run('sips', ['-c', String(squareSide), String(squareSide), sourcePath, '--out', croppedPath])
  }
  run('sips', ['-z', String(size), String(size), inputPath, '--out', outputPath])
  fs.rmSync(croppedPath, { force: true })
}

function writeSvgWrapper(pngPath, outputPath) {
  const base64 = fs.readFileSync(pngPath).toString('base64')
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="512" height="512" viewBox="0 0 512 512"><rect width="512" height="512" rx="96" fill="#fff"/><image href="data:image/png;base64,${base64}" x="0" y="0" width="512" height="512" preserveAspectRatio="xMidYMid meet"/></svg>\n`
  fs.writeFileSync(outputPath, svg)
}

function iconPixelSize(image) {
  const points = Number.parseFloat(String(image.size || '').split('x')[0])
  const scale = Number.parseFloat(String(image.scale || '1x').replace('x', ''))
  if (!Number.isFinite(points) || !Number.isFinite(scale)) return 0
  return Math.round(points * scale)
}

function writeIosIcons(sourcePng, iconSetDir) {
  const contentsPath = path.join(iconSetDir, 'Contents.json')
  if (!fs.existsSync(contentsPath)) fail(`未找到 iOS 图标清单：${contentsPath}`)
  const contents = JSON.parse(fs.readFileSync(contentsPath, 'utf8'))
  for (const image of contents.images || []) {
    if (!image.filename) continue
    const pixels = iconPixelSize(image)
    if (!pixels) continue
    makeSquarePng(sourcePng, path.join(iconSetDir, image.filename), pixels)
  }
}

function writeDistAssets(config, sourcePng, distDir) {
  ensureDirectory(distDir)
  const appLogoPath = path.join(distDir, 'app-logo.png')
  makeSquarePng(sourcePng, appLogoPath, 512)
  writeSvgWrapper(appLogoPath, path.join(distDir, 'logo.svg'))
  fs.writeFileSync(
    path.join(distDir, 'mobile-branding.json'),
    `${JSON.stringify({
      platformName: config.platformName,
      logoImageUrl: '/app-logo.png',
      originalLogoImageUrl: config.logoImageUrl,
      intro: config.intro,
    }, null, 2)}\n`,
  )
}

async function main() {
  const options = parseArgs(process.argv.slice(2))
  const workDir = options.workDir || fs.mkdtempSync(path.join(os.tmpdir(), 'hongfu-branding-'))
  ensureDirectory(workDir)

  const config = normalizeConfig(await fetchJson(`${options.apiBase}/api/user/mobile/site-config`))
  if (!config.logoImageUrl) {
    if (options.distDir) {
      ensureDirectory(options.distDir)
      fs.writeFileSync(path.join(options.distDir, 'mobile-branding.json'), `${JSON.stringify(config, null, 2)}\n`)
    }
    console.warn('后台站点配置没有 Logo 图片，本次不会生成 App 图标。')
    return
  }

  const downloadedPath = path.join(workDir, 'brand-logo-source')
  const normalizedPngPath = path.join(workDir, 'brand-logo.png')
  await downloadFile(config.logoImageUrl, downloadedPath)
  convertToPng(downloadedPath, normalizedPngPath)

  if (options.distDir) writeDistAssets(config, normalizedPngPath, options.distDir)
  if (options.iosAppIconDir) writeIosIcons(normalizedPngPath, options.iosAppIconDir)

  console.log(`已同步打包品牌资源：${config.platformName || '未配置平台名称'} ${config.logoImageUrl}`)
}

main().catch((error) => {
  fail(error instanceof Error ? error.message : String(error))
})
