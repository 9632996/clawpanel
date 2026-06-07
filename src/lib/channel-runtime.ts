// @ts-nocheck
const EMPTY_STATUS = Object.freeze({
  supported: false,
  ts: 0,
  partial: false,
  warnings: Object.freeze([]),
  channelOrder: Object.freeze([]),
  channelLabels: Object.freeze({}),
  channelDetailLabels: Object.freeze({}),
  channelAccounts: Object.freeze({}),
  channelDefaultAccountId: Object.freeze({}),
  channels: Object.freeze({}),
  eventLoop: null,
})

export function normalizeChannelRuntimeStatus(raw) {
  if (!raw || typeof raw !== 'object') {
    return { ...EMPTY_STATUS }
  }

  return {
    supported: true,
    ts: Number.isFinite(raw.ts) ? raw.ts : Date.now(),
    partial: raw.partial === true,
    warnings: Array.isArray(raw.warnings) ? raw.warnings.filter(Boolean).map(String) : [],
    channelOrder: Array.isArray(raw.channelOrder) ? raw.channelOrder.filter(Boolean).map(String) : [],
    channelLabels: plainObject(raw.channelLabels),
    channelDetailLabels: plainObject(raw.channelDetailLabels),
    channelAccounts: normalizeChannelAccounts(raw.channelAccounts),
    channelDefaultAccountId: plainObject(raw.channelDefaultAccountId),
    channels: plainObject(raw.channels),
    eventLoop: raw.eventLoop && typeof raw.eventLoop === 'object' ? raw.eventLoop : null,
  }
}

export function getChannelRuntimeSummary(status, channelId, fallbackLabel = '') {
  const normalized = status && typeof status === 'object' && Object.prototype.hasOwnProperty.call(status, 'supported')
    ? status
    : normalizeChannelRuntimeStatus(status)
  const channel = String(channelId || '')
  const accounts = normalizeChannelAccounts(normalized.channelAccounts)[channel] || []
  const counts = countAccountStates(accounts)
  const supported = normalized.supported === true
  const state = supported ? pickSummaryState({ accounts, counts }) : 'unsupported'

  return {
    supported,
    channel,
    label: normalized.channelLabels?.[channel] || fallbackLabel || channel,
    detailLabel: normalized.channelDetailLabels?.[channel] || '',
    defaultAccountId: normalized.channelDefaultAccountId?.[channel] || '',
    rawChannel: normalized.channels?.[channel] || null,
    accounts,
    counts,
    state,
    lastError: firstAccountValue(accounts, 'lastError'),
    lastInboundAt: latestNumber(accounts, 'lastInboundAt'),
    lastOutboundAt: latestNumber(accounts, 'lastOutboundAt'),
    lastTransportActivityAt: latestNumber(accounts, 'lastTransportActivityAt'),
  }
}

export function getRuntimeStateMeta(state) {
  switch (state) {
    case 'error':
      return { label: '异常', tone: 'error', icon: 'alert-triangle' }
    case 'connected':
      return { label: '已连接', tone: 'success', icon: 'check' }
    case 'running':
      return { label: '运行中', tone: 'accent', icon: 'play' }
    case 'configured':
      return { label: '已配置', tone: 'warning', icon: 'gear' }
    case 'disabled':
      return { label: '已停用', tone: 'muted', icon: 'pause' }
    case 'missing':
      return { label: '未配置', tone: 'muted', icon: 'circle' }
    case 'unsupported':
      return { label: '内核不支持', tone: 'warning', icon: 'alert-triangle' }
    default:
      return { label: '未知', tone: 'muted', icon: 'info' }
  }
}

export function formatRuntimeAge(value, now = Date.now()) {
  const ts = Number(value)
  if (!Number.isFinite(ts) || ts <= 0) return ''
  const delta = Math.max(0, Number(now) - ts)
  const seconds = Math.floor(delta / 1000)
  if (seconds < 45) return '刚刚'
  const minutes = Math.floor(seconds / 60)
  if (minutes < 60) return `${minutes} 分钟前`
  const hours = Math.floor(minutes / 60)
  if (hours < 48) return `${hours} 小时前`
  const days = Math.floor(hours / 24)
  return `${days} 天前`
}

function normalizeChannelAccounts(value) {
  const result = {}
  if (!value || typeof value !== 'object' || Array.isArray(value)) return result
  for (const [channel, accounts] of Object.entries(value)) {
    if (!Array.isArray(accounts)) continue
    result[channel] = accounts
      .filter(account => account && typeof account === 'object')
      .map(account => {
        const accountId = account.accountId == null || account.accountId === '' ? 'default' : String(account.accountId)
        return {
          ...account,
          accountId,
          state: pickAccountState(account),
          lastError: account.lastError ? String(account.lastError) : '',
        }
      })
  }
  return result
}

function plainObject(value) {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return {}
  return { ...value }
}

function pickAccountState(account) {
  if (account.lastError) return 'error'
  if (account.enabled === false) return 'disabled'
  if (account.connected === true || account.linked === true) return 'connected'
  if (account.running === true) return 'running'
  if (account.configured === true) return 'configured'
  return 'missing'
}

function pickSummaryState({ accounts, counts }) {
  if (!accounts.length) return 'missing'
  if (counts.error > 0) return 'error'
  if (counts.connected > 0) return 'connected'
  if (counts.running > 0) return 'running'
  if (counts.configured > 0) return 'configured'
  if (counts.disabled > 0) return 'disabled'
  return 'missing'
}

function countAccountStates(accounts) {
  const counts = {
    total: accounts.length,
    error: 0,
    connected: 0,
    running: 0,
    configured: 0,
    disabled: 0,
    missing: 0,
  }
  for (const account of accounts) {
    const state = account.state || pickAccountState(account)
    counts[state] = (counts[state] || 0) + 1
  }
  return counts
}

function latestNumber(accounts, key) {
  let latest = 0
  for (const account of accounts) {
    const value = Number(account?.[key])
    if (Number.isFinite(value) && value > latest) latest = value
  }
  return latest || null
}

function firstAccountValue(accounts, key) {
  const found = accounts.find(account => account?.[key])
  return found?.[key] ? String(found[key]) : ''
}
