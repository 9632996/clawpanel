/**
 * CodeWhale 引擎
 * 原生支持 DeepSeek/MiMo 等 Chat Completions 模型的编码智能体。
 */
import type { EnginePlugin, EngineNavSection, EngineRoute } from './types'

const CODEWHALE_ICON = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" width="16" height="16"><path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2z"/><path d="M8 12l3 3 5-5"/></svg>'

let _ready = false
let _installed = false
const _listeners: Array<() => void> = []

const engine: EnginePlugin = {
  id: 'codewhale',
  name: 'CodeWhale',
  description: '支持 DeepSeek/MiMo 的原生编码智能体',
  icon: CODEWHALE_ICON,

  async detect() {
    try {
      const { api } = await import('../../lib/tauri-api.ts')
      const cfg = await api.readPanelConfig() as Record<string, unknown> | null
      const cw = cfg?.codewhale as Record<string, unknown> | undefined
      _installed = !!cw?.cliPath
      _ready = _installed
    } catch {
      _installed = false
      _ready = false
    }
    return { installed: _installed, ready: _ready }
  },

  async boot() {
    _ready = _installed
  },

  cleanup() {},

  getNavItems(): EngineNavSection[] {
    return [
      {
        section: '监控',
        items: [
          { route: '/w/dashboard', label: '控制台', icon: 'dashboard' },
          { route: '/w/chat', label: '编码对话', icon: 'chat' },
        ],
      },
      {
        section: '管理',
        items: [
          { route: '/w/providers', label: '提供商管理', icon: 'models' },
          { route: '/w/skills', label: '技能管理', icon: 'skills' },
          { route: '/w/settings', label: '设置', icon: 'settings' },
        ],
      },
      {
        section: '',
        items: [
          { route: '/about', label: '关于', icon: 'about' },
        ],
      },
    ]
  },

  getRoutes(): EngineRoute[] {
    return [
      { path: '/w/dashboard', loader: () => import('./pages/dashboard.ts') },
      { path: '/w/chat', loader: () => import('./pages/chat.ts') },
      { path: '/w/providers', loader: () => import('./pages/providers.ts') },
      { path: '/w/skills', loader: () => import('./pages/skills.ts') },
      { path: '/w/settings', loader: () => import('./pages/settings.ts') },
      { path: '/about', loader: () => import('../../pages/about.ts') },
    ]
  },

  getSetupRoute: () => '/w/dashboard',
  getDefaultRoute: () => '/w/chat',
  isReady: () => _ready,
  isGatewayRunning: () => false,
  isGatewayForeign: () => false,

  onStateChange(fn: () => void) {
    _listeners.push(fn)
    return () => {
      const idx = _listeners.indexOf(fn)
      if (idx >= 0) _listeners.splice(idx, 1)
    }
  },

  onReadyChange(fn: () => void) {
    _listeners.push(fn)
    return () => {
      const idx = _listeners.indexOf(fn)
      if (idx >= 0) _listeners.splice(idx, 1)
    }
  },

  isFeatureAvailable: () => true,
}

export default engine
