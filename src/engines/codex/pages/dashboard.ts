// @ts-nocheck
import { api } from '../../../lib/tauri-api.ts'

const PROVIDER_LABELS = {
  deepseek: 'DeepSeek',
  'xiaomi-mimo': '小米 MiMo',
  qwen: '通义千问',
  zhipu: '智谱 GLM',
  baidu: '百度文心',
  moonshot: '月之暗面',
  aizuopin: 'Aizuopin 中转',
}

export async function render() {
  const page = document.createElement('div')
  page.className = 'page codex-dashboard'
  page.innerHTML = `
    <div class="page-header">
      <div>
        <h1>Codex 控制台</h1>
        <p class="page-desc">支持国内大模型的编码智能体，通过通用协议转换适配 Chat Completions API。</p>
      </div>
    </div>
    <div class="codex-grid">
      <section class="codex-panel">
        <div class="codex-panel-title">运行状态</div>
        <div class="codex-status-row">
          <span>二进制</span>
          <strong data-codex-bin>检测中</strong>
        </div>
        <div class="codex-status-row">
          <span>当前提供商</span>
          <strong data-codex-provider>-</strong>
        </div>
        <div class="codex-status-row">
          <span>当前模型</span>
          <strong data-codex-model>-</strong>
        </div>
        <div class="codex-status-row">
          <span>协议模式</span>
          <strong data-codex-wire>-</strong>
        </div>
      </section>
      <section class="codex-panel">
        <div class="codex-panel-title">可用提供商</div>
        <div class="codex-providers-list" data-codex-providers>
          <div>加载中...</div>
        </div>
      </section>
      <section class="codex-panel">
        <div class="codex-panel-title">内置技能</div>
        <div class="codex-skills-count" data-codex-skills>
          <div>加载中...</div>
        </div>
      </section>
    </div>
  `

  try {
    const [cfg, status] = await Promise.all([
      api.readPanelConfig(),
      api.codexStatus().catch(() => null),
    ])
    const codex = cfg?.codex || {}

    // 二进制状态
    page.querySelector('[data-codex-bin]').textContent = status?.cliExists
      ? '已部署'
      : `未部署${status?.cliPath ? `: ${status.cliPath}` : ''}`

    // 当前提供商
    const provider = status?.provider || codex.provider || 'aizuopin'
    page.querySelector('[data-codex-provider]').textContent = PROVIDER_LABELS[provider] || provider
    page.querySelector('[data-codex-model]').textContent = status?.model || codex.model || '-'

    // 协议模式
    const isDirect = provider === 'aizuopin'
    page.querySelector('[data-codex-wire]').textContent = isDirect ? 'Responses API（直连）' : 'Chat Completions（协议转换）'

    // 提供商列表
    const providers = codex.providers || {}
    const providerHtml = Object.entries(providers).map(([id, p]) => {
      const active = id === provider ? ' <span class="codex-badge">当前</span>' : ''
      const mode = id === 'aizuopin' ? 'Responses' : 'Chat Completions'
      return `<div class="codex-provider-item"><strong>${PROVIDER_LABELS[id] || p.name}</strong>${active}<br><small>${mode} · ${p.baseUrl || ''}</small></div>`
    }).join('')
    page.querySelector('[data-codex-providers]').innerHTML = providerHtml || '<div>无预置提供商</div>'

    // 技能数量
    page.querySelector('[data-codex-skills]').innerHTML = `<div>${status?.skillCount ?? 0} 个编码技能已预置</div>`
  } catch {
    page.querySelector('[data-codex-bin]').textContent = '等待构建器注入'
  }

  return page
}
