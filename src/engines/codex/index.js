/**
 * Codex 引擎
 * 产品基座先以源码级内置入口接入，后续在 vendor/codex 上做国内模型适配。
 */
import { t } from '../../lib/i18n.js'

const CODEX_ICON = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" width="16" height="16"><path d="M8 4L3 9l5 5"/><path d="M16 4l5 5-5 5"/><path d="M14 3l-4 18"/></svg>'

let _ready = true
let _listeners = []

export default {
  id: 'codex',
  name: 'Codex',
  description: '源码级定制的本地编码智能体',
  icon: CODEX_ICON,

  async detect() {
    return { installed: true, ready: _ready }
  },

  async boot() {
    _ready = true
  },

  cleanup() {},

  getNavItems() {
    return [{
      section: t('sidebar.sectionMonitor'),
      items: [
        { route: '/c/dashboard', label: 'Codex 控制台', icon: 'dashboard' },
      ],
    }, {
      section: '',
      items: [
        { route: '/assistant', label: t('sidebar.assistant'), icon: 'assistant' },
        { route: '/settings', label: t('sidebar.settings'), icon: 'settings' },
        { route: '/about', label: t('sidebar.about'), icon: 'about' },
      ],
    }]
  },

  getRoutes() {
    return [
      { path: '/c/dashboard', loader: () => import('./pages/dashboard.js') },
      { path: '/assistant', loader: () => import('../../pages/assistant.js') },
      { path: '/settings', loader: () => import('../../pages/settings.js') },
      { path: '/about', loader: () => import('../../pages/about.js') },
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
