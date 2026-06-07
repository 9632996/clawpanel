import { defineConfig } from 'vite'
import { devApiPlugin, readJsonFileRelaxed } from './scripts/dev-api.js'
import fs from 'fs'
import path from 'path'
import { homedir } from 'os'

// 读取 package.json 版本号，构建时注入前端
const pkg = JSON.parse(fs.readFileSync(new URL('./package.json', import.meta.url), 'utf8'))
const zhizhuaServiceUrl = (process.env.VITE_ZHIZHUA_SERVICE_URL || 'https://ai.iazp.cn').replace(/\/+$/, '')
const zhizhuaModelBaseUrl = (process.env.VITE_ZHIZHUA_MODEL_BASE_URL || `${zhizhuaServiceUrl}/v1`).replace(/\/+$/, '')

function zhizhuaUrlPlugin() {
  const replacements = new Map([
    ['https://ai.iazp.cn/v1', zhizhuaModelBaseUrl],
    ['https://ai.iazp.cn', zhizhuaServiceUrl],
    ['ai.iazp.cn', zhizhuaServiceUrl.replace(/^https?:\/\//, '')],
  ])
  return {
    name: 'zhizhua-url-config',
    transform(code, id) {
      if (!/\.(js|ts|html)$/.test(id)) return null
      let next = code
      for (const [from, to] of replacements) next = next.split(from).join(to)
      return next === code ? null : { code: next, map: null }
    },
    transformIndexHtml(html) {
      let next = html
      for (const [from, to] of replacements) next = next.split(from).join(to)
      return next
    },
  }
}

// 读取 Gateway 端口（启动时读取一次）
// 注意：Gateway 默认端口是 18789，不是 18790
let gatewayPort = 18789
try {
  const cfgPath = path.join(homedir(), '.openclaw', 'openclaw.json')
  if (fs.existsSync(cfgPath)) {
    const cfg = readJsonFileRelaxed(cfgPath)
    // 端口必须 > 0 且 < 65536
    const port = cfg?.gateway?.port
    if (port && typeof port === 'number' && port > 0 && port < 65536) {
      gatewayPort = port
    }
  }
} catch (e) {
  console.warn('[vite] 读取 Gateway 端口配置失败，使用默认端口 18789:', e.message)
}

console.log(`[vite] Gateway WebSocket 代理目标: ws://127.0.0.1:${gatewayPort}`)
console.log(`[vite] Zhizhua service URL: ${zhizhuaServiceUrl}`)
console.log(`[vite] Zhizhua model base URL: ${zhizhuaModelBaseUrl}`)

export default defineConfig({
  plugins: [zhizhuaUrlPlugin(), devApiPlugin()],
  define: {
    __APP_VERSION__: JSON.stringify(pkg.version),
    __ZHIZHUA_SERVICE_URL__: JSON.stringify(zhizhuaServiceUrl),
    __ZHIZHUA_MODEL_BASE_URL__: JSON.stringify(zhizhuaModelBaseUrl),
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    proxy: {
      '/ws': {
        target: `ws://127.0.0.1:${gatewayPort}`,
        ws: true,
        changeOrigin: true,
        timeout: 30000,
        configure: (proxy, options) => {
          proxy.on('proxyReqWs', (proxyReq, req, socket) => {
            socket.setTimeout(30000)
            socket.on('timeout', () => {
              console.warn('[vite/ws] WebSocket 超时，关闭连接')
              socket.destroy()
            })
          })
          proxy.on('error', (err, req, socket) => {
            console.warn(`[vite/ws] 代理错误: ${err.code} ${err.message}`)
            // WebSocket 升级后 socket 是 net.Socket，无 headersSent
            if (socket && !socket.destroyed) {
              socket.destroy()
            }
          })
        },
      },
    },
  },
  envPrefix: ['VITE_', 'TAURI_'],
  build: {
    target: ['es2021', 'chrome100', 'safari13'],
    minify: !process.env.TAURI_DEBUG ? 'esbuild' : false,
    sourcemap: !!process.env.TAURI_DEBUG,
  },
})
