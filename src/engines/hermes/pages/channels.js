/**
 * Hermes Agent 渠道配置
 */
import { t } from '../../../lib/i18n.js'
import { api } from '../../../lib/tauri-api.js'
import { toast } from '../../../components/toast.js'
import { humanizeErrorText } from '../../../lib/humanize-error.js'
import { icon } from '../../../lib/icons.js'

const CHANNELS = [
  {
    id: 'telegram',
    icon: 'message-circle',
    titleKey: 'engine.hermesChannelTelegram',
    descKey: 'engine.hermesChannelTelegramDesc',
    secretFields: ['botToken'],
    fields: [
      { key: 'botToken', labelKey: 'engine.hermesChannelBotToken', type: 'password', placeholder: '123456:ABC-DEF...' },
    ],
    advancedFields: [
      { key: 'replyToMode', labelKey: 'engine.hermesChannelTelegramReplyToMode', type: 'select', options: [['first', 'engine.hermesChannelTelegramReplyFirst'], ['all', 'engine.hermesChannelTelegramReplyAll'], ['off', 'engine.hermesChannelTelegramReplyOff']] },
    ],
    advancedToggles: [
      { key: 'guestMode', labelKey: 'engine.hermesChannelTelegramGuestMode' },
      { key: 'disableLinkPreviews', labelKey: 'engine.hermesChannelTelegramDisableLinkPreviews' },
    ],
  },
  {
    id: 'discord',
    icon: 'message-square',
    titleKey: 'engine.hermesChannelDiscord',
    descKey: 'engine.hermesChannelDiscordDesc',
    secretFields: ['token'],
    fields: [
      { key: 'token', labelKey: 'engine.hermesChannelBotToken', type: 'password', placeholder: 'MTA...' },
      { key: 'homeChannel', labelKey: 'engine.hermesChannelDiscordHomeChannel', type: 'text', placeholder: '123456789012345678' },
      { key: 'homeChannelName', labelKey: 'engine.hermesChannelDiscordHomeChannelName', type: 'text', placeholder: 'ops' },
    ],
    advancedFields: [
      { key: 'freeResponseChannels', labelKey: 'engine.hermesChannelDiscordFreeResponseChannels', type: 'textarea', placeholderKey: 'engine.hermesChannelDiscordFreeResponseChannelsPh' },
      { key: 'allowedChannels', labelKey: 'engine.hermesChannelDiscordAllowedChannels', type: 'textarea', placeholderKey: 'engine.hermesChannelDiscordChannelListPh' },
      { key: 'ignoredChannels', labelKey: 'engine.hermesChannelDiscordIgnoredChannels', type: 'textarea', placeholderKey: 'engine.hermesChannelDiscordChannelListPh' },
      { key: 'noThreadChannels', labelKey: 'engine.hermesChannelDiscordNoThreadChannels', type: 'textarea', placeholderKey: 'engine.hermesChannelDiscordChannelListPh' },
      { key: 'historyBackfillLimit', labelKey: 'engine.hermesChannelDiscordHistoryBackfillLimit', type: 'text', placeholder: '12' },
      { key: 'replyToMode', labelKey: 'engine.hermesChannelDiscordReplyToMode', type: 'select', options: [['first', 'engine.hermesChannelDiscordReplyFirst'], ['all', 'engine.hermesChannelDiscordReplyAll'], ['off', 'engine.hermesChannelDiscordReplyOff']] },
    ],
    advancedToggles: [
      { key: 'autoThread', labelKey: 'engine.hermesChannelDiscordAutoThread' },
      { key: 'reactions', labelKey: 'engine.hermesChannelDiscordReactions' },
      { key: 'threadRequireMention', labelKey: 'engine.hermesChannelDiscordThreadRequireMention' },
      { key: 'historyBackfill', labelKey: 'engine.hermesChannelDiscordHistoryBackfill' },
    ],
  },
  {
    id: 'slack',
    icon: 'hash',
    titleKey: 'engine.hermesChannelSlack',
    descKey: 'engine.hermesChannelSlackDesc',
    secretFields: ['botToken', 'appToken', 'signingSecret'],
    fields: [
      { key: 'botToken', labelKey: 'engine.hermesChannelSlackBotToken', type: 'password', placeholder: 'xoxb-...' },
      { key: 'appToken', labelKey: 'engine.hermesChannelSlackAppToken', type: 'password', placeholder: 'xapp-...' },
      { key: 'signingSecret', labelKey: 'engine.hermesChannelSigningSecret', type: 'password', placeholder: 'optional' },
      { key: 'webhookPath', labelKey: 'engine.hermesChannelWebhookPath', type: 'text', placeholder: '/slack/events' },
    ],
  },
  {
    id: 'feishu',
    icon: 'send',
    titleKey: 'engine.hermesChannelFeishu',
    descKey: 'engine.hermesChannelFeishuDesc',
    secretFields: ['appSecret'],
    fields: [
      { key: 'appId', labelKey: 'engine.hermesChannelFeishuAppId', type: 'text', placeholder: 'cli_xxx' },
      { key: 'appSecret', labelKey: 'engine.hermesChannelFeishuAppSecret', type: 'password', placeholder: 'app secret' },
      { key: 'domain', labelKey: 'engine.hermesChannelFeishuDomain', type: 'select', options: [['feishu', 'engine.hermesChannelFeishuDomainCn'], ['lark', 'engine.hermesChannelFeishuDomainIntl']] },
      { key: 'connectionMode', labelKey: 'engine.hermesChannelConnectionMode', type: 'select', options: [['websocket', 'WebSocket'], ['webhook', 'Webhook']] },
      { key: 'webhookPath', labelKey: 'engine.hermesChannelWebhookPath', type: 'text', placeholder: '/feishu/webhook' },
      { key: 'reactionNotifications', labelKey: 'engine.hermesChannelReactions', type: 'select', options: [['off', 'engine.hermesChannelReactionsOff'], ['basic', 'engine.hermesChannelReactionsBasic']] },
    ],
    toggles: [
      { key: 'typingIndicator', labelKey: 'engine.hermesChannelTypingIndicator' },
      { key: 'resolveSenderNames', labelKey: 'engine.hermesChannelResolveSenderNames' },
    ],
  },
  {
    id: 'dingtalk',
    icon: 'message-circle',
    titleKey: 'engine.hermesChannelDingTalk',
    descKey: 'engine.hermesChannelDingTalkDesc',
    secretFields: ['clientSecret'],
    fields: [
      { key: 'clientId', labelKey: 'engine.hermesChannelDingTalkClientId', type: 'text', placeholder: 'dingxxxxxx' },
      { key: 'clientSecret', labelKey: 'engine.hermesChannelDingTalkClientSecret', type: 'password', placeholder: 'client secret' },
    ],
  },
  {
    id: 'teams',
    icon: 'users',
    titleKey: 'engine.hermesChannelTeams',
    descKey: 'engine.hermesChannelTeamsDesc',
    secretFields: ['clientId', 'clientSecret', 'tenantId'],
    fields: [
      { key: 'clientId', labelKey: 'engine.hermesChannelTeamsClientId', type: 'text', placeholder: '00000000-0000-0000-0000-000000000000' },
      { key: 'clientSecret', labelKey: 'engine.hermesChannelTeamsClientSecret', type: 'password', placeholder: 'client secret' },
      { key: 'tenantId', labelKey: 'engine.hermesChannelTeamsTenantId', type: 'text', placeholder: '00000000-0000-0000-0000-000000000000' },
      { key: 'port', labelKey: 'engine.hermesChannelPort', type: 'number', placeholder: '3978' },
      { key: 'serviceUrl', labelKey: 'engine.hermesChannelServiceUrl', type: 'url', placeholder: 'https://smba.trafficmanager.net/teams/' },
    ],
    policyFields: [
      { key: 'allowFrom', labelKey: 'engine.hermesChannelAllowedUsers', type: 'textarea', placeholderKey: 'engine.hermesChannelTeamsAllowedUsersPh' },
      { key: 'allowAllUsers', labelKey: 'engine.hermesChannelAllowAllUsers', type: 'checkbox' },
      { key: 'homeChannel', labelKey: 'engine.hermesChannelHomeChannel', type: 'text', placeholder: '19:xxx@thread.tacv2' },
      { key: 'homeChannelName', labelKey: 'engine.hermesChannelHomeChannelName', type: 'text', placeholder: 'ops' },
    ],
  },
  {
    id: 'google_chat',
    icon: 'message-square',
    titleKey: 'engine.hermesChannelGoogleChat',
    descKey: 'engine.hermesChannelGoogleChatDesc',
    secretFields: ['projectId', 'serviceAccountJson'],
    fields: [
      { key: 'projectId', labelKey: 'engine.hermesChannelGoogleProjectId', type: 'text', placeholder: 'my-gcp-project' },
      { key: 'subscriptionName', labelKey: 'engine.hermesChannelGoogleSubscriptionName', type: 'text', placeholder: 'projects/my-gcp-project/subscriptions/hermes' },
      { key: 'serviceAccountJson', labelKey: 'engine.hermesChannelGoogleServiceAccount', type: 'password', placeholderKey: 'engine.hermesChannelGoogleServiceAccountPh' },
    ],
    policyFields: [
      { key: 'allowFrom', labelKey: 'engine.hermesChannelAllowedUsers', type: 'textarea', placeholderKey: 'engine.hermesChannelGoogleAllowedUsersPh' },
      { key: 'allowAllUsers', labelKey: 'engine.hermesChannelAllowAllUsers', type: 'checkbox' },
      { key: 'homeChannel', labelKey: 'engine.hermesChannelHomeChannel', type: 'text', placeholder: 'spaces/AAAA...' },
      { key: 'homeChannelName', labelKey: 'engine.hermesChannelHomeChannelName', type: 'text', placeholder: 'ops-space' },
    ],
  },
  {
    id: 'irc',
    icon: 'hash',
    titleKey: 'engine.hermesChannelIrc',
    descKey: 'engine.hermesChannelIrcDesc',
    secretFields: ['server', 'serverPassword', 'nickservPassword'],
    fields: [
      { key: 'server', labelKey: 'engine.hermesChannelIrcServer', type: 'text', placeholder: 'irc.libera.chat' },
      { key: 'port', labelKey: 'engine.hermesChannelPort', type: 'number', placeholder: '6697' },
      { key: 'nickname', labelKey: 'engine.hermesChannelIrcNickname', type: 'text', placeholder: 'hermes-bot' },
      { key: 'channel', labelKey: 'engine.hermesChannelIrcChannel', type: 'text', placeholder: '#hermes' },
      { key: 'serverPassword', labelKey: 'engine.hermesChannelIrcServerPassword', type: 'password', placeholder: 'optional' },
      { key: 'nickservPassword', labelKey: 'engine.hermesChannelIrcNickservPassword', type: 'password', placeholder: 'optional' },
    ],
    toggles: [
      { key: 'useTls', labelKey: 'engine.hermesChannelIrcUseTls' },
    ],
    policyFields: [
      { key: 'allowFrom', labelKey: 'engine.hermesChannelAllowedUsers', type: 'textarea', placeholderKey: 'engine.hermesChannelIrcAllowedUsersPh' },
      { key: 'allowAllUsers', labelKey: 'engine.hermesChannelAllowAllUsers', type: 'checkbox' },
      { key: 'homeChannel', labelKey: 'engine.hermesChannelHomeChannel', type: 'text', placeholder: '#reports' },
      { key: 'homeChannelName', labelKey: 'engine.hermesChannelHomeChannelName', type: 'text', placeholder: 'reports' },
    ],
  },
  {
    id: 'line',
    icon: 'message-circle',
    titleKey: 'engine.hermesChannelLine',
    descKey: 'engine.hermesChannelLineDesc',
    secretFields: ['channelAccessToken', 'channelSecret'],
    fields: [
      { key: 'channelAccessToken', labelKey: 'engine.hermesChannelLineAccessToken', type: 'password', placeholder: 'LINE channel access token' },
      { key: 'channelSecret', labelKey: 'engine.hermesChannelLineSecret', type: 'password', placeholder: 'LINE channel secret' },
      { key: 'port', labelKey: 'engine.hermesChannelPort', type: 'number', placeholder: '8646' },
      { key: 'host', labelKey: 'engine.hermesChannelHost', type: 'text', placeholder: '0.0.0.0' },
      { key: 'publicUrl', labelKey: 'engine.hermesChannelPublicUrl', type: 'url', placeholder: 'https://line.example.com' },
    ],
    policyFields: [
      { key: 'allowFrom', labelKey: 'engine.hermesChannelAllowedUsers', type: 'textarea', placeholderKey: 'engine.hermesChannelLineAllowedUsersPh' },
      { key: 'allowedGroups', labelKey: 'engine.hermesChannelLineAllowedGroups', type: 'textarea', placeholderKey: 'engine.hermesChannelLineAllowedGroupsPh' },
      { key: 'allowedRooms', labelKey: 'engine.hermesChannelLineAllowedRooms', type: 'textarea', placeholderKey: 'engine.hermesChannelLineAllowedRoomsPh' },
      { key: 'allowAllUsers', labelKey: 'engine.hermesChannelAllowAllUsers', type: 'checkbox' },
      { key: 'homeChannel', labelKey: 'engine.hermesChannelHomeChannel', type: 'text', placeholder: 'Uxxxxxxxx' },
      { key: 'slowResponseThreshold', labelKey: 'engine.hermesChannelLineSlowResponse', type: 'number', placeholder: '45' },
    ],
  },
  {
    id: 'simplex',
    icon: 'radio',
    titleKey: 'engine.hermesChannelSimpleX',
    descKey: 'engine.hermesChannelSimpleXDesc',
    secretFields: ['wsUrl'],
    fields: [
      { key: 'wsUrl', labelKey: 'engine.hermesChannelSimpleXWsUrl', type: 'url', placeholder: 'ws://127.0.0.1:5225' },
    ],
    policyFields: [
      { key: 'allowFrom', labelKey: 'engine.hermesChannelAllowedUsers', type: 'textarea', placeholderKey: 'engine.hermesChannelSimpleXAllowedUsersPh' },
      { key: 'allowAllUsers', labelKey: 'engine.hermesChannelAllowAllUsers', type: 'checkbox' },
      { key: 'homeChannel', labelKey: 'engine.hermesChannelHomeChannel', type: 'text', placeholder: 'group:ops' },
      { key: 'homeChannelName', labelKey: 'engine.hermesChannelHomeChannelName', type: 'text', placeholder: 'Ops' },
    ],
  },
]

const LEGACY_POLICY_FIELDS = [
  { key: 'dmPolicy', labelKey: 'engine.hermesChannelDmPolicy', type: 'select', options: [['pair', 'engine.hermesChannelPolicyPair'], ['open', 'engine.hermesChannelPolicyOpen'], ['allowlist', 'engine.hermesChannelPolicyAllowlist'], ['disabled', 'engine.hermesChannelPolicyDisabled']] },
  { key: 'groupPolicy', labelKey: 'engine.hermesChannelGroupPolicy', type: 'select', options: [['allowlist', 'engine.hermesChannelPolicyAllowlist'], ['open', 'engine.hermesChannelPolicyOpen'], ['disabled', 'engine.hermesChannelPolicyDisabled']] },
  { key: 'requireMention', labelKey: 'engine.hermesChannelRequireMention', type: 'checkbox' },
  { key: 'allowFrom', labelKey: 'engine.hermesChannelAllowFrom', type: 'textarea', placeholderKey: 'engine.hermesChannelAllowFromPlaceholder' },
  { key: 'groupAllowFrom', labelKey: 'engine.hermesChannelGroupAllowFrom', type: 'textarea', placeholderKey: 'engine.hermesChannelGroupAllowFromPlaceholder' },
]

const DISPLAY_FIELDS = [
  {
    key: 'displayToolProgress',
    labelKey: 'engine.hermesChannelDisplayToolProgress',
    type: 'select',
    options: [
      ['off', 'engine.hermesChannelDisplayToolProgressOff'],
      ['new', 'engine.hermesChannelDisplayToolProgressNew'],
      ['all', 'engine.hermesChannelDisplayToolProgressAll'],
      ['verbose', 'engine.hermesChannelDisplayToolProgressVerbose'],
    ],
  },
  {
    key: 'displayStreaming',
    labelKey: 'engine.hermesChannelDisplayStreaming',
    type: 'select',
    options: [
      ['inherit', 'engine.hermesChannelDisplayStreamingInherit'],
      ['true', 'engine.hermesChannelDisplayStreamingOn'],
      ['false', 'engine.hermesChannelDisplayStreamingOff'],
    ],
  },
  { key: 'displayToolPreviewLength', labelKey: 'engine.hermesChannelDisplayToolPreviewLength', type: 'number', placeholder: '0' },
]

const DISPLAY_TOGGLES = [
  { key: 'displayShowReasoning', labelKey: 'engine.hermesChannelDisplayShowReasoning', type: 'checkbox' },
  { key: 'displayCleanupProgress', labelKey: 'engine.hermesChannelDisplayCleanupProgress', type: 'checkbox' },
]

function esc(value) {
  return String(value ?? '')
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
}

function channelMeta(id) {
  return CHANNELS.find(channel => channel.id === id) || CHANNELS[0]
}

function defaultForm(platform) {
  const channel = channelMeta(platform)
  const form = {
    enabled: false,
    displayToolProgress: 'all',
    displayShowReasoning: false,
    displayToolPreviewLength: 0,
    displayStreaming: 'inherit',
    displayCleanupProgress: false,
  }
  if (!channel.policyFields) {
    form.dmPolicy = 'pair'
    form.groupPolicy = 'allowlist'
    form.allowFrom = ''
    form.groupAllowFrom = ''
    form.requireMention = true
  }
  if (platform === 'feishu') {
    form.domain = 'feishu'
    form.connectionMode = 'websocket'
    form.webhookPath = '/feishu/webhook'
    form.reactionNotifications = 'off'
    form.typingIndicator = true
    form.resolveSenderNames = true
  }
  if (platform === 'discord') {
    form.autoThread = true
    form.reactions = true
    form.threadRequireMention = false
    form.historyBackfill = false
    form.replyToMode = 'first'
  }
  if (platform === 'telegram') {
    form.replyToMode = 'first'
    form.guestMode = false
    form.disableLinkPreviews = false
  }
  if (platform === 'slack') form.webhookPath = '/slack/events'
  return form
}

function normalizeForm(platform, form = {}) {
  return { ...defaultForm(platform), ...(form || {}) }
}

function valueOf(form, key) {
  const value = form?.[key]
  return value == null ? '' : String(value)
}

function isConfigured(channel, form) {
  return channel.secretFields.some(key => valueOf(form, key).trim())
}

function renderField(field, form, disabled) {
  const value = valueOf(form, field.key)
  const label = esc(t(field.labelKey))
  const placeholder = field.placeholderKey ? t(field.placeholderKey) : (field.placeholder || '')
  if (field.type === 'checkbox') {
    return `
      <label class="hm-channel-check">
        <input class="hm-channel-input" data-key="${esc(field.key)}" type="checkbox" ${form[field.key] ? 'checked' : ''} ${disabled ? 'disabled' : ''}>
        <span>${label}</span>
      </label>
    `
  }
  if (field.type === 'select') {
    return `
      <label class="hm-field">
        <span class="hm-field-label">${label}</span>
        <select class="hm-input hm-channel-input" data-key="${esc(field.key)}" ${disabled ? 'disabled' : ''}>
          ${(field.options || []).map(([optionValue, optionLabel]) => `
            <option value="${esc(optionValue)}" ${value === optionValue ? 'selected' : ''}>${esc(optionLabel.startsWith('engine.') ? t(optionLabel) : optionLabel)}</option>
          `).join('')}
        </select>
      </label>
    `
  }
  if (field.type === 'textarea') {
    return `
      <label class="hm-field">
        <span class="hm-field-label">${label}</span>
        <textarea class="hm-input hm-channel-input hm-channel-textarea" data-key="${esc(field.key)}" ${disabled ? 'disabled' : ''} placeholder="${esc(placeholder)}">${esc(value)}</textarea>
      </label>
    `
  }
  return `
    <label class="hm-field">
      <span class="hm-field-label">${label}</span>
      <input class="hm-input hm-channel-input" data-key="${esc(field.key)}" type="${esc(field.type || 'text')}" value="${esc(value)}" ${disabled ? 'disabled' : ''} placeholder="${esc(placeholder)}" autocomplete="off">
    </label>
  `
}

function collectForm(el, platform) {
  const form = normalizeForm(platform, {})
  el.querySelectorAll('.hm-channel-input').forEach(input => {
    const key = input.dataset.key
    if (!key) return
    if (input.type === 'checkbox') form[key] = input.checked
    else form[key] = input.value
  })
  return form
}

export function render() {
  const el = document.createElement('div')
  el.className = 'page hm-channels-page'
  el.dataset.engine = 'hermes'

  let active = 'telegram'
  let values = {}
  let configPath = ''
  let loading = true
  let saving = false
  let error = ''
  let success = ''

  function draw() {
    const channel = channelMeta(active)
    const form = normalizeForm(active, values[active])
    const disabled = loading || saving
    const policyFields = channel.policyFields || LEGACY_POLICY_FIELDS
    const policyInputs = policyFields.filter(field => field.type !== 'checkbox')
    const policyToggles = policyFields.filter(field => field.type === 'checkbox')
    const enabledCount = CHANNELS.filter(item => normalizeForm(item.id, values[item.id]).enabled).length
    const configuredCount = CHANNELS.filter(item => isConfigured(item, normalizeForm(item.id, values[item.id]))).length

    el.innerHTML = `
      <div class="hm-hero">
        <div class="hm-hero-title">
          <div class="hm-hero-eyebrow">${esc(t('engine.hermesChannelsEyebrow'))}</div>
          <h1 class="hm-hero-h1">${esc(t('engine.hermesChannelsTitle'))}</h1>
          <div class="hm-hero-sub">${esc(configPath || '~/.hermes/config.yaml')}</div>
        </div>
        <div class="hm-hero-actions">
          <button class="hm-btn hm-btn--ghost hm-btn--sm" id="hm-channels-reload" ${disabled ? 'disabled' : ''}>${icon('refresh-cw', 14)}${esc(t('engine.hermesConfigReload'))}</button>
          <button class="hm-btn hm-btn--cta hm-btn--sm" id="hm-channels-save" ${disabled ? 'disabled' : ''}>${saving ? esc(t('engine.hermesChannelSaving')) : esc(t('engine.hermesChannelSave'))}</button>
        </div>
      </div>

      <section class="hm-channel-summary" aria-label="${esc(t('engine.hermesChannelSummary'))}">
        <div class="hm-channel-stat"><span>${esc(t('engine.hermesChannelEnabledCount'))}</span><strong>${enabledCount}</strong></div>
        <div class="hm-channel-stat"><span>${esc(t('engine.hermesChannelConfiguredCount'))}</span><strong>${configuredCount}</strong></div>
        <div class="hm-channel-stat"><span>${esc(t('engine.hermesChannelRuntimeWrite'))}</span><strong>${esc(t('engine.hermesChannelRuntimeWriteValue'))}</strong></div>
      </section>

      ${(error || success) ? `
        <div class="hm-channel-alert ${error ? 'is-error' : 'is-success'}">
          ${icon(error ? 'alert-triangle' : 'check-circle', 15)}
          <span>${esc(error || success)}</span>
        </div>
      ` : ''}

      <div class="hm-channel-layout">
        <section class="hm-panel hm-channel-list-panel">
          <div class="hm-panel-header">
            <div class="hm-panel-title">${esc(t('engine.hermesChannelPlatforms'))}</div>
          </div>
          <div class="hm-panel-body hm-panel-body--tight">
            <div class="hm-channel-list" role="tablist" aria-label="${esc(t('engine.hermesChannelPlatforms'))}">
              ${CHANNELS.map(item => {
                const itemForm = normalizeForm(item.id, values[item.id])
                return `
                  <button class="hm-channel-tab ${item.id === active ? 'is-active' : ''}" data-channel="${esc(item.id)}" role="tab" aria-selected="${item.id === active ? 'true' : 'false'}" ${disabled ? 'disabled' : ''}>
                    <span class="hm-channel-tab-icon">${icon(item.icon, 16)}</span>
                    <span class="hm-channel-tab-main">
                      <strong>${esc(t(item.titleKey))}</strong>
                      <small>${esc(itemForm.enabled ? t('engine.hermesChannelEnabled') : t('engine.hermesChannelDisabled'))}</small>
                    </span>
                    <span class="hm-channel-dot ${itemForm.enabled ? 'is-on' : ''}" aria-hidden="true"></span>
                  </button>
                `
              }).join('')}
            </div>
          </div>
        </section>

        <section class="hm-panel hm-channel-form-panel">
          <div class="hm-panel-header">
            <div>
              <div class="hm-panel-title">${icon(channel.icon, 15)}${esc(t(channel.titleKey))}</div>
              <div class="hm-channel-panel-desc">${esc(t(channel.descKey))}</div>
            </div>
            <label class="hm-channel-switch">
              <input class="hm-channel-input" data-key="enabled" type="checkbox" ${form.enabled ? 'checked' : ''} ${disabled ? 'disabled' : ''}>
              <span>${esc(form.enabled ? t('engine.hermesChannelEnabled') : t('engine.hermesChannelDisabled'))}</span>
            </label>
          </div>
          <div class="hm-panel-body">
            ${loading ? `
              <div class="hm-channel-loading">${esc(t('common.loading'))}...</div>
            ` : `
              <div class="hm-channel-section">
                <div class="hm-channel-section-title">${esc(t('engine.hermesChannelCredentials'))}</div>
                <div class="hm-field-row">
                  ${channel.fields.map(field => renderField(field, form, disabled)).join('')}
                </div>
                ${(channel.toggles || []).length ? `
                  <div class="hm-channel-toggle-grid">
                    ${channel.toggles.map(toggle => `
                      <label class="hm-channel-check">
                        <input class="hm-channel-input" data-key="${esc(toggle.key)}" type="checkbox" ${form[toggle.key] ? 'checked' : ''} ${disabled ? 'disabled' : ''}>
                        <span>${esc(t(toggle.labelKey))}</span>
                      </label>
                    `).join('')}
                  </div>
                ` : ''}
              </div>

              <div class="hm-channel-section">
                <div class="hm-channel-section-title">${esc(t('engine.hermesChannelAccessPolicy'))}</div>
                ${policyInputs.length ? `
                  <div class="hm-field-row">
                    ${policyInputs.map(field => renderField(field, form, disabled)).join('')}
                  </div>
                ` : ''}
                ${policyToggles.length ? `
                  <div class="hm-channel-toggle-grid">
                    ${policyToggles.map(field => renderField(field, form, disabled)).join('')}
                  </div>
                ` : ''}
              </div>

              <div class="hm-channel-section">
                <div>
                  <div class="hm-channel-section-title">${esc(t('engine.hermesChannelDisplayBehavior'))}</div>
                  <div class="hm-channel-section-hint">${esc(t('engine.hermesChannelDisplayHint'))}</div>
                </div>
                <div class="hm-field-row">
                  ${DISPLAY_FIELDS.map(field => renderField(field, form, disabled)).join('')}
                </div>
                <div class="hm-channel-toggle-grid">
                  ${DISPLAY_TOGGLES.map(field => renderField(field, form, disabled)).join('')}
                </div>
              </div>

              ${(channel.advancedFields || []).length ? `
                <div class="hm-channel-section">
                  <div class="hm-channel-section-title">${esc(t('engine.hermesChannelRuntimeBehavior'))}</div>
                  ${(channel.advancedToggles || []).length ? `
                    <div class="hm-channel-toggle-grid">
                      ${channel.advancedToggles.map(toggle => `
                        <label class="hm-channel-check">
                          <input class="hm-channel-input" data-key="${esc(toggle.key)}" type="checkbox" ${form[toggle.key] ? 'checked' : ''} ${disabled ? 'disabled' : ''}>
                          <span>${esc(t(toggle.labelKey))}</span>
                        </label>
                      `).join('')}
                    </div>
                  ` : ''}
                  <div class="hm-field-row">
                    ${channel.advancedFields.map(field => renderField(field, form, disabled)).join('')}
                  </div>
                </div>
              ` : ''}

              <div class="hm-channel-footnote">
                ${icon('info', 14)}
                <span>${esc(t('engine.hermesChannelRestartHint'))}</span>
              </div>
            `}
          </div>
        </section>
      </div>
    `

    el.querySelector('#hm-channels-reload')?.addEventListener('click', load)
    el.querySelector('#hm-channels-save')?.addEventListener('click', save)
    el.querySelectorAll('.hm-channel-tab').forEach(button => {
      button.addEventListener('click', () => {
        if (!loading && !saving) values = { ...values, [active]: collectForm(el, active) }
        active = button.dataset.channel || active
        error = ''
        success = ''
        draw()
      })
    })
  }

  async function load() {
    loading = true
    error = ''
    success = ''
    draw()
    try {
      const data = await api.hermesChannelConfigRead()
      values = data?.values || {}
      configPath = data?.configPath || ''
    } catch (err) {
      error = humanizeErrorText(err, t('engine.hermesChannelLoadFailed'))
    } finally {
      loading = false
      draw()
    }
  }

  async function save() {
    const form = collectForm(el, active)
    values = { ...values, [active]: form }
    saving = true
    error = ''
    success = ''
    draw()
    try {
      const result = await api.hermesChannelConfigSave(active, form)
      values = { ...values, [active]: result?.values || form }
      success = t('engine.hermesChannelSaved')
      toast(success, 'success')
    } catch (err) {
      error = humanizeErrorText(err, t('engine.hermesChannelSaveFailed'))
      toast(error, 'error')
    } finally {
      saving = false
      draw()
    }
  }

  draw()
  load()
  return el
}
