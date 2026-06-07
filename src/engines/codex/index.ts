// @ts-nocheck
/**
 * Codex 引擎
 * 支持国内模型（DeepSeek、MiMo、通义千问、智谱GLM 等）的编码智能体。
 */
import { t } from '../../lib/i18n.ts'

const CODEX_ICON = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" width="16" height="16"><path d="M8 4L3 9l5 5"/><path d="M16 4l5 5-5 5"/><path d="M14 3l-4 18"/></svg>'

let _ready = false
let _installed = false
let _listeners = []

export default {
  id: 'codex',
  name: 'Codex',
  description: '支持国内大模型的本地编码智能体',
  icon: CODEX_ICON,

  async detect() {
    try {
      const cfg = await import('../../lib/tauri-api.ts').then(m => m.api.readPanelConfig())
      const binPath = cfg?.codex?.cliPath
      _installed = !!binPath
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

  getNavItems() {
    return [{
      section: t('sidebar.sectionMonitor'),
      items: [
        { route: '/c/dashboard', label: 'Codex 控制台', icon: 'dashboard' },
        { route: '/c/chat', label: '原生 App', icon: 'chat' },
      ],
    }, {
      section: '管理',
      items: [
        { route: '/c/models', label: '模型管理', icon: 'models' },
        { route: '/c/skills', label: '技能管理', icon: 'skills' },
      ],
    }, {
      section: '',
      items: [
        { route: '/settings', label: t('sidebar.settings'), icon: 'settings' },
        { route: '/about', label: t('sidebar.about'), icon: 'about' },
      ],
    }]
  },

  getRoutes() {
    return [
      { path: '/c/dashboard', loader: () => import('./pages/dashboard.ts') },
      { path: '/c/chat', loader: () => import('./pages/chat.ts') },
      { path: '/c/models', loader: () => import('./pages/models.ts') },
      { path: '/c/skills', loader: () => import('./pages/skills.ts') },
      { path: '/settings', loader: () => import('../../pages/settings.ts') },
      { path: '/about', loader: () => import('../../pages/about.ts') },
    ]
  },

  getSetupRoute() { return '/c/dashboard' },
  getDefaultRoute() { return '/c/dashboard' },

  isReady() { return _ready },
  isGatewayRunning() { return false },
  isGatewayForeign() { return false },

  onStateChange(fn) {
    _listeners.push(fn)
    return () => { _listeners = _listeners.filter(cb => cb !== fn) }
  },
  onReadyChange(fn) {
    _listeners.push(fn)
    return () => { _listeners = _listeners.filter(cb => cb !== fn) }
  },

  isFeatureAvailable() { return true },
}
