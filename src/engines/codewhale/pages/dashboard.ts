import { api } from '../../../lib/tauri-api.ts'
import type { CodeWhaleStatus, PanelConfig, ProviderConfig } from '../types'

const PROVIDER_LABELS: Record<string, string> = {
  deepseek: 'DeepSeek',
  'xiaomi-mimo': '小米 MiMo',
  qwen: '通义千问',
  zhipu: '智谱 GLM',
  moonshot: '月之暗面',
  aizuopin: 'Aizuopin 中转',
  'nvidia-nim': 'NVIDIA NIM',
  openrouter: 'OpenRouter',
  siliconflow: 'SiliconFlow',
  ollama: 'Ollama（本地）',
}

function esc(value: unknown): string {
  return String(value ?? '').replace(/[&<>"]/g, ch =>
    ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' } as Record<string, string>)[ch] ?? ch
  )
}

export async function render(): Promise<HTMLElement> {
  const page = document.createElement('div')
  page.className = 'page codewhale-dashboard'

  let status: CodeWhaleStatus | null = null
  let cfg: PanelConfig | null = null

  try {
    status = await api.codewhaleStatus() as CodeWhaleStatus
    cfg = await api.readPanelConfig() as PanelConfig
  } catch { /* ignore */ }

  const cw = cfg?.codewhale
  const provider = cw?.provider ?? 'deepseek'
  const model = cw?.model ?? '-'
  const providers = cw?.providers ?? {}

  const statusClass = status?.ready ? 'cw-status-ready' : 'cw-status-error'
  const statusText = status?.ready
    ? `就绪 · ${status.version ?? 'v0.8'} · ${status.skillCount} 技能`
    : `未就绪 · ${status?.cliExists === false ? '缺少二进制' : '缺少配置'}`

  const providerEntries = Object.entries(providers) as [string, ProviderConfig][]
  const providerListHtml = providerEntries.length > 0
    ? providerEntries.map(([id, p]) => {
        const active = id === provider
        return `<div class="cw-provider-item${active ? ' cw-provider-active' : ''}">
          <strong>${esc(PROVIDER_LABELS[id] ?? p.name)}</strong>
          ${active ? '<span class="cw-badge">当前</span>' : ''}
          <br><small>Chat Completions · ${esc(p.baseUrl)}</small>
        </div>`
      }).join('')
    : '<div class="cw-muted">使用 CodeWhale 内置 17 个提供商</div>'

  page.innerHTML = `
    <div class="page-header">
      <div>
        <h1>CodeWhale 控制台</h1>
        <p class="page-desc">原生支持 DeepSeek/MiMo 等国内模型的编码智能体，基于 Chat Completions API。</p>
      </div>
    </div>
    <div class="cw-grid">
      <section class="cw-card">
        <div class="cw-card-title">运行状态</div>
        <div class="cw-status-grid">
          <div class="cw-stat">
            <span class="cw-stat-label">引擎</span>
            <span class="cw-status-badge ${statusClass}">${esc(statusText)}</span>
          </div>
          <div class="cw-stat">
            <span class="cw-stat-label">当前提供商</span>
            <strong>${esc(PROVIDER_LABELS[provider] ?? provider)}</strong>
          </div>
          <div class="cw-stat">
            <span class="cw-stat-label">当前模型</span>
            <strong>${esc(model)}</strong>
          </div>
          <div class="cw-stat">
            <span class="cw-stat-label">协议</span>
            <span>Chat Completions</span>
          </div>
          <div class="cw-stat">
            <span class="cw-stat-label">二进制</span>
            <span>${status?.cliExists ? '已部署' : '未部署'}</span>
          </div>
          <div class="cw-stat">
            <span class="cw-stat-label">TUI</span>
            <span>${status?.tuiExists ? '已部署' : '未部署'}</span>
          </div>
        </div>
      </section>
      <section class="cw-card">
        <div class="cw-card-title">可用提供商</div>
        <div class="cw-provider-list">${providerListHtml}</div>
      </section>
      <section class="cw-card cw-card-wide">
        <div class="cw-card-title">快速开始</div>
        <div class="cw-quickstart">
          <p>1. 在 <strong>设置</strong> 中配置 API Key（DeepSeek / MiMo 等）</p>
          <p>2. 选择提供商和模型</p>
          <p>3. 进入 <strong>编码对话</strong> 开始交互</p>
        </div>
      </section>
    </div>
  `

  return page
}
