// @ts-nocheck
import { api } from '../../../lib/tauri-api.ts'
import { toast } from '../../../components/toast.ts'
import { navigate } from '../../../router.ts'
import { writeGatewayConfig } from '../../../lib/app-state.ts'

const DEFAULT_MODEL = {
  full: 'aizuopin/gpt-5.4',
  providerKey: 'aizuopin',
  modelId: 'gpt-5.4',
  providerName: 'aizuopin',
  baseUrl: 'https://ai.iazp.cn/v1',
  apiType: 'openai-completions',
  apiKey: '$env:AIZUOPIN_API_KEY',
}

const TOOL_LABELS = {
  codex: 'Codex CLI',
  openclaw: 'OpenClaw',
  codewhale: 'CodeWhale',
  hermes: 'Hermes Agent',
}

function escapeHtml(value) {
  if (value == null) return ''
  return String(value)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
}

function apiKeyDisplayValue(value, providerKey) {
  if (!value) return providerKey ? `$env:${providerKey.replace(/[-.]/g, '_').toUpperCase()}_API_KEY` : ''
  if (typeof value === 'string') return value
  if (typeof value === 'object') {
    const envName = value.$env || (value.source === 'env' ? value.id || value.env : '')
    if (envName) return `$env:${envName}`
    if (value.$ref) return `$ref:${value.$ref}`
    return JSON.stringify(value)
  }
  return String(value)
}

function modelId(model) {
  return typeof model === 'string' ? model : (model?.id || model?.name || '')
}

function collectPlatformModels(config) {
  const providers = config?.models?.providers || {}
  const items = []
  for (const [providerKey, provider] of Object.entries(providers)) {
    for (const rawModel of provider.models || []) {
      const id = modelId(rawModel)
      if (!id) continue
      items.push({
        full: `${providerKey}/${id}`,
        providerKey,
        modelId: id,
        providerName: provider.name || providerKey,
        baseUrl: provider.baseUrl || '',
        apiType: provider.api || 'openai-completions',
        apiKey: apiKeyDisplayValue(provider.apiKey || '', providerKey),
      })
    }
  }
  if (!items.some(item => item.full === DEFAULT_MODEL.full)) items.unshift(DEFAULT_MODEL)
  return items
}

function findSelectedModel(models, full) {
  return models.find(item => item.full === full) || models[0] || DEFAULT_MODEL
}

function currentModelFull(panelConfig, status) {
  const provider = status?.provider || panelConfig?.codex?.provider || DEFAULT_MODEL.providerKey
  const model = status?.model || panelConfig?.codex?.model || DEFAULT_MODEL.modelId
  return `${provider}/${model}`
}

function renderModelOptions(models, selectedFull) {
  return models.map(item => `
    <option value="${escapeHtml(item.full)}" ${item.full === selectedFull ? 'selected' : ''}>
      ${escapeHtml(item.full)} · ${escapeHtml(item.baseUrl || '-')}
    </option>
  `).join('')
}

function renderToolChecks(tools) {
  const order = ['codex', 'openclaw', 'codewhale', 'hermes']
  const sorted = [...tools].sort((a, b) => order.indexOf(a.id) - order.indexOf(b.id))
  return sorted.map(tool => `
    <label class="codex-tool-check">
      <input type="checkbox" value="${escapeHtml(tool.id)}" ${tool.id === 'codex' ? 'checked' : ''}>
      <span>
        <strong>${escapeHtml(TOOL_LABELS[tool.id] || tool.name || tool.id)}</strong>
        <small>${escapeHtml((tool.configFiles || []).join('、'))}</small>
      </span>
    </label>
  `).join('')
}

function buildModelInfo(item, form) {
  return {
    id: `${form.provider || item.providerKey}/${form.model || item.modelId}`,
    name: form.provider || item.providerName || item.providerKey,
    provider: form.provider || item.providerKey,
    baseUrl: form.baseUrl || item.baseUrl,
    apiKey: form.apiKey || item.apiKey || '',
    model: form.model || item.modelId,
    protocol: 'openai-completions',
    relayMode: true,
  }
}

function readForm(page) {
  return {
    provider: page.querySelector('[data-codex-provider-input]')?.value.trim() || '',
    model: page.querySelector('[data-codex-model-input]')?.value.trim() || '',
    baseUrl: page.querySelector('[data-codex-baseurl-input]')?.value.trim() || '',
    apiKey: page.querySelector('[data-codex-apikey-input]')?.value.trim() || '',
    apiType: 'openai-completions',
  }
}

function apiKeyConfigValue(value, providerKey) {
  const trimmed = String(value || '').trim()
  if (!trimmed) return { $env: `${providerKey.replace(/[-.]/g, '_').toUpperCase()}_API_KEY` }
  if (trimmed.startsWith('$env:')) return { $env: trimmed.slice(5).trim() }
  if (trimmed.startsWith('$ref:')) return { $ref: trimmed.slice(5).trim() }
  return trimmed
}

function ensureProvider(config, form) {
  if (!config.models || typeof config.models !== 'object') config.models = {}
  if (!config.models.providers || typeof config.models.providers !== 'object') config.models.providers = {}
  const providerKey = form.provider || DEFAULT_MODEL.providerKey
  const provider = config.models.providers[providerKey] || {}
  provider.name = provider.name || providerKey
  provider.baseUrl = form.baseUrl || provider.baseUrl || DEFAULT_MODEL.baseUrl
  provider.api = 'openai-completions'
  provider.apiKey = apiKeyConfigValue(form.apiKey, providerKey)
  provider.models = Array.isArray(provider.models) ? provider.models : []
  config.models.providers[providerKey] = provider
  return { providerKey, provider }
}

function modelExists(models, id) {
  return models.some(model => modelId(model) === id)
}

function refreshModelSelect(page, state, selectedFull) {
  state.models = collectPlatformModels(state.config)
  const selected = findSelectedModel(state.models, selectedFull)
  page.querySelector('[data-codex-model-select]').innerHTML = renderModelOptions(state.models, selected.full)
  page.querySelector('[data-codex-model-select]').value = selected.full
  fillForm(page, selected)
}

function fillForm(page, item) {
  page.querySelector('[data-codex-provider-input]').value = item.providerKey || ''
  page.querySelector('[data-codex-model-input]').value = item.modelId || ''
  page.querySelector('[data-codex-baseurl-input]').value = item.baseUrl || ''
  page.querySelector('[data-codex-apikey-input]').value = item.apiKey || ''
}

function renderCurrent(page, panelConfig, status) {
  const codex = panelConfig?.codex || {}
  page.querySelector('[data-current-provider]').textContent = status?.provider || codex.provider || '-'
  page.querySelector('[data-current-model]').textContent = status?.model || codex.model || '-'
  page.querySelector('[data-current-baseurl]').textContent = codex.baseUrl || '-'
  page.querySelector('[data-current-cli]').textContent = status?.cliExists ? '已部署' : `未部署${status?.cliPath ? `: ${status.cliPath}` : ''}`
}

async function applyToTools(page, state, toolIds) {
  const selected = findSelectedModel(state.models, page.querySelector('[data-codex-model-select]').value)
  const modelInfo = buildModelInfo(selected, readForm(page))
  if (!modelInfo.provider || !modelInfo.model || !modelInfo.baseUrl) {
    toast('Provider、模型和 Base URL 不能为空', 'warning')
    return
  }
  if (!toolIds.length) {
    toast('请选择至少一个工具', 'warning')
    return
  }
  const ok = []
  const failed = []
  for (const toolId of toolIds) {
    try {
      await api.applyModelToTool(toolId, modelInfo)
      ok.push(toolId)
    } catch (e) {
      failed.push(`${TOOL_LABELS[toolId] || toolId}: ${e?.message || e}`)
    }
  }
  if (failed.length) {
    toast(`已应用 ${ok.length} 个，失败 ${failed.length} 个：${failed.join('；')}`, 'warning', { duration: 9000 })
  } else {
    toast(`已应用到 ${ok.map(id => TOOL_LABELS[id] || id).join('、')}`, 'success')
  }
  try {
    const [panelConfig, status] = await Promise.all([
      api.readPanelConfig(),
      api.codexStatus().catch(() => null),
    ])
    renderCurrent(page, panelConfig, status)
  } catch {}
}

async function syncRemoteModels(page, state) {
  const form = readForm(page)
  if (!form.provider || !form.baseUrl) {
    toast('Provider 和 Base URL 不能为空', 'warning')
    return
  }
  const remoteIds = await api.listRemoteModels(form.baseUrl, form.apiKey || '', 'openai-completions')
  if (!Array.isArray(remoteIds) || !remoteIds.length) {
    toast('服务商没有返回可用模型', 'warning')
    return
  }
  const config = await api.readOpenclawConfig().catch(() => state.config || {})
  const { providerKey, provider } = ensureProvider(config, form)
  let added = 0
  for (const id of remoteIds) {
    const cleanId = String(id || '').trim()
    if (!cleanId || modelExists(provider.models, cleanId)) continue
    provider.models.push({ id: cleanId, input: ['text'] })
    added += 1
  }
  state.config = config
  await writeGatewayConfig(config, { apply: false, reason: 'codex-sync-models' })
  const selectedModel = form.model && remoteIds.includes(form.model) ? form.model : (remoteIds[0] || form.model || DEFAULT_MODEL.modelId)
  refreshModelSelect(page, state, `${providerKey}/${selectedModel}`)
  toast(added ? `已同步 ${remoteIds.length} 个模型，新增 ${added} 个` : `已同步 ${remoteIds.length} 个模型，没有新增项`, added ? 'success' : 'info')
}

export async function render() {
  const page = document.createElement('div')
  page.className = 'page codex-models'
  page.innerHTML = `
    <div class="page-header codex-models-header">
      <div>
        <h1>Codex 模型管理</h1>
        <p class="page-desc">统一使用智爪平台模型配置，直接写入 Codex CLI 运行配置。</p>
      </div>
      <button class="btn btn-secondary btn-sm" data-open-platform-models>打开平台模型中心</button>
    </div>

    <div class="codex-model-layout">
      <section class="codex-panel">
        <div class="codex-panel-title">当前 Codex 配置</div>
        <div class="codex-status-row"><span>Provider</span><strong data-current-provider>-</strong></div>
        <div class="codex-status-row"><span>模型</span><strong data-current-model>-</strong></div>
        <div class="codex-status-row"><span>Base URL</span><strong data-current-baseurl>-</strong></div>
        <div class="codex-status-row"><span>CLI</span><strong data-current-cli>-</strong></div>
      </section>

      <section class="codex-panel codex-panel-wide">
        <div class="codex-panel-title">选择并应用模型</div>
        <div class="codex-model-form">
          <label>
            <span>平台模型</span>
            <select class="form-input" data-codex-model-select></select>
          </label>
          <label>
            <span>Provider</span>
            <input class="form-input" data-codex-provider-input placeholder="aizuopin">
          </label>
          <label>
            <span>模型 ID</span>
            <input class="form-input" data-codex-model-input placeholder="gpt-5.4">
          </label>
          <label>
            <span>Base URL</span>
            <input class="form-input" data-codex-baseurl-input placeholder="https://ai.iazp.cn/v1">
          </label>
          <label>
            <span>API Key / 环境变量</span>
            <input class="form-input" data-codex-apikey-input type="password" placeholder="$env:AIZUOPIN_API_KEY">
          </label>
        </div>
        <div class="codex-model-actions">
          <button class="btn btn-secondary btn-sm" data-sync-models>同步模型</button>
          <button class="btn btn-primary btn-sm" data-apply-codex>应用到 Codex</button>
          <button class="btn btn-secondary btn-sm" data-apply-selected-tools>同步到勾选工具</button>
        </div>
      </section>

      <section class="codex-panel codex-panel-wide">
        <div class="codex-panel-title">同步范围</div>
        <div class="codex-tool-grid" data-tool-list>
          <div class="codex-muted">加载中...</div>
        </div>
      </section>
    </div>
  `

  const state = { config: null, models: [DEFAULT_MODEL], tools: [] }

  try {
    const [openclawConfig, panelConfig, status, tools] = await Promise.all([
      api.readOpenclawConfig().catch(() => null),
      api.readPanelConfig().catch(() => null),
      api.codexStatus().catch(() => null),
      api.listModelTools().catch(() => []),
    ])
    state.config = openclawConfig || {}
    state.models = collectPlatformModels(state.config)
    state.tools = tools || []
    const selectedFull = currentModelFull(panelConfig, status)
    const selected = findSelectedModel(state.models, selectedFull)
    page.querySelector('[data-codex-model-select]').innerHTML = renderModelOptions(state.models, selected.full)
    page.querySelector('[data-tool-list]').innerHTML = renderToolChecks(state.tools)
    fillForm(page, selected)
    renderCurrent(page, panelConfig, status)
  } catch (e) {
    state.config = {}
    page.querySelector('[data-codex-model-select]').innerHTML = renderModelOptions([DEFAULT_MODEL], DEFAULT_MODEL.full)
    page.querySelector('[data-tool-list]').innerHTML = '<div class="codex-muted">工具模板加载失败</div>'
    fillForm(page, DEFAULT_MODEL)
    toast(`加载 Codex 模型配置失败: ${e?.message || e}`, 'error')
  }

  page.querySelector('[data-codex-model-select]')?.addEventListener('change', e => {
    fillForm(page, findSelectedModel(state.models, e.target.value))
  })
  page.querySelector('[data-open-platform-models]')?.addEventListener('click', () => navigate('/models'))
  page.querySelector('[data-sync-models]')?.addEventListener('click', async e => {
    const btn = e.currentTarget
    btn.disabled = true
    btn.textContent = '同步中'
    try { await syncRemoteModels(page, state) } catch (err) { toast(`同步模型失败: ${err?.message || err}`, 'error', { duration: 8000 }) } finally { btn.disabled = false; btn.textContent = '同步模型' }
  })
  page.querySelector('[data-apply-codex]')?.addEventListener('click', async e => {
    const btn = e.currentTarget
    btn.disabled = true
    btn.textContent = '应用中'
    try { await applyToTools(page, state, ['codex']) } finally { btn.disabled = false; btn.textContent = '应用到 Codex' }
  })
  page.querySelector('[data-apply-selected-tools]')?.addEventListener('click', async e => {
    const selected = [...page.querySelectorAll('[data-tool-list] input:checked')].map(input => input.value)
    const btn = e.currentTarget
    btn.disabled = true
    btn.textContent = '同步中'
    try { await applyToTools(page, state, selected) } finally { btn.disabled = false; btn.textContent = '同步到勾选工具' }
  })

  return page
}
