// @ts-nocheck
/**
 * OpenClaw 引擎
 * 包装现有 OpenClaw 逻辑为统一的 Engine 接口，不改动原有代码
 */
import { detectOpenclawStatus, isOpenclawReady, isGatewayRunning, isGatewayForeign,
         onGatewayChange, startGatewayPoll, stopGatewayPoll, onReadyChange } from '../../lib/app-state.ts'
import { initFeatureGates, isFeatureAvailable } from '../../lib/feature-gates.ts'
import { t } from '../../lib/i18n.ts'

export default {
  id: 'openclaw',
  name: 'OpenClaw',
  description: 'OpenClaw AI Agent Framework',
  icon: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="16" height="16"><path d="M12 2L2 7l10 5 10-5-10-5z"/><path d="M2 17l10 5 10-5"/><path d="M2 12l10 5 10-5"/></svg>',

  /** 检测 OpenClaw 是否已安装 */
  async detect() {
    const ready = await detectOpenclawStatus()
    return { installed: ready, ready }
  },

  /** 启动 OpenClaw 引擎相关逻辑 */
  async boot() {
    await detectOpenclawStatus()
    await initFeatureGates().catch(() => {})
    startGatewayPoll()
  },

  /** 清理（停止轮询等） */
  cleanup() {
    stopGatewayPoll()
  },

  /** 侧边栏菜单项 */
  getNavItems() {
    if (!isOpenclawReady()) {
      return [{
        section: '',
        items: [
          { route: '/setup', label: t('sidebar.setup'), icon: 'setup' },
          { route: '/assistant', label: t('sidebar.assistant'), icon: 'assistant' },
        ]
      }, {
        section: '',
        items: [
          { route: '/settings', label: t('sidebar.settings'), icon: 'settings' },
          { route: '/chat-debug', label: t('sidebar.chatDebug'), icon: 'debug' },
          { route: '/about', label: t('sidebar.about'), icon: 'about' },
        ]
      }]
    }
    return [{
      section: t('sidebar.sectionMonitor'),
      items: [
        { route: '/dashboard', label: t('sidebar.dashboard'), icon: 'dashboard' },
        { route: '/assistant', label: t('sidebar.assistant'), icon: 'assistant' },
        { route: '/chat', label: t('sidebar.chat'), icon: 'chat' },
        { route: '/route-map', label: t('sidebar.routeMap'), icon: 'route-map' },
        { route: '/services', label: t('sidebar.services'), icon: 'services' },
        { route: '/logs', label: t('sidebar.logs'), icon: 'logs' },
      ]
    }, {
      section: t('sidebar.sectionConfig'),
      items: [
        { route: '/models', label: t('sidebar.models'), icon: 'models' },
        { route: '/agents', label: t('sidebar.agents'), icon: 'agents' },
        { route: '/gateway', label: t('sidebar.gateway'), icon: 'gateway' },
        { route: '/channels', label: t('sidebar.channels'), icon: 'channels' },
        { route: '/communication', label: t('sidebar.communication'), icon: 'settings' },
        { route: '/notifications', label: t('sidebar.notifications'), icon: 'channels' },
        { route: '/security', label: t('sidebar.security'), icon: 'security' },
      ]
    }, {
      section: t('sidebar.sectionData'),
      items: [
        { route: '/memory', label: t('sidebar.memory'), icon: 'memory', gate: 'memory' },
        { route: '/dreaming', label: t('sidebar.dreaming'), icon: 'dreaming', gate: 'dreaming' },
        { route: '/cron', label: t('sidebar.cron'), icon: 'clock', gate: 'cron' },
        { route: '/usage', label: t('sidebar.usage'), icon: 'bar-chart' },
      ]
    }, {
      section: t('sidebar.sectionExtension'),
      items: [
        { route: '/skills', label: t('sidebar.skills'), icon: 'skills', gate: 'skills' },
        { route: '/plugin-hub', label: t('sidebar.pluginHub'), icon: 'extensions' },
      ]
    }, {
      section: '',
      items: [
        { route: '/settings', label: t('sidebar.settings'), icon: 'settings' },
        { route: '/chat-debug', label: t('sidebar.checkRepair'), icon: 'diagnose' },
        { route: '/glossary', label: t('sidebar.glossary'), icon: 'about' },
        { route: '/about', label: t('sidebar.about'), icon: 'about' },
      ]
    }]
  },

  /** 路由注册表 */
  getRoutes() {
    return [
      { path: '/dashboard', loader: () => import('../../pages/dashboard.ts') },
      { path: '/chat', loader: () => import('../../pages/chat.ts') },
      { path: '/chat-debug', loader: () => import('../../pages/chat-debug.ts') },
      { path: '/services', loader: () => import('../../pages/services.ts') },
      { path: '/logs', loader: () => import('../../pages/logs.ts') },
      { path: '/models', loader: () => import('../../pages/models.ts') },
      { path: '/agents', loader: () => import('../../pages/agents.ts') },
      { path: '/agent-detail', loader: () => import('../../pages/agent-detail.ts') },
      { path: '/gateway', loader: () => import('../../pages/gateway.ts') },
      { path: '/memory', loader: () => import('../../pages/memory.ts') },
      { path: '/dreaming', loader: () => import('../../pages/dreaming.ts') },
      { path: '/skills', loader: () => import('../../pages/skills.ts') },
      { path: '/security', loader: () => import('../../pages/security.ts') },
      { path: '/about', loader: () => import('../../pages/about.ts') },
      { path: '/assistant', loader: () => import('../../pages/assistant.ts') },
      { path: '/setup', loader: () => import('../../pages/setup.ts') },
      { path: '/channels', loader: () => import('../../pages/channels.ts') },
      { path: '/cron', loader: () => import('../../pages/cron.ts') },
      { path: '/usage', loader: () => import('../../pages/usage.ts') },
      { path: '/communication', loader: () => import('../../pages/communication.ts') },
      { path: '/notifications', loader: () => import('../../pages/notifications.ts') },
      { path: '/settings', loader: () => import('../../pages/settings.ts') },
      { path: '/route-map', loader: () => import('../../pages/route-map.ts') },
      { path: '/plugin-hub', loader: () => import('../../pages/plugin-hub.ts') },
      { path: '/diagnose', loader: () => import('../../pages/chat-debug.ts') },
      { path: '/glossary', loader: () => import('../../pages/glossary.ts') },
    ]
  },

  getSetupRoute() { return '/setup' },
  getDefaultRoute() { return '/dashboard' },

  isReady() { return isOpenclawReady() },
  isGatewayRunning() { return isGatewayRunning() },
  isGatewayForeign() { return isGatewayForeign() },

  onStateChange(fn) { return onGatewayChange(fn) },
  onReadyChange(fn) { return onReadyChange(fn) },

  /** 功能门控：基于 OpenClaw 版本号 */
  isFeatureAvailable(featureId) { return isFeatureAvailable(featureId) },
}
