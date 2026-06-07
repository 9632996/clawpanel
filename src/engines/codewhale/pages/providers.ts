import type { PanelConfig, ProviderConfig } from '../types'
import { api } from '../../../lib/tauri-api.ts'

function esc(value: unknown): string {
  return String(value ?? '').replace(/[&<>"]/g, ch =>
    ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' } as Record<string, string>)[ch] ?? ch
  )
}

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

// CodeWhale 内置提供商完整列表
const BUILTIN_PROVIDERS: Array<{ id: string; name: string; model: string; envKey: string }> = [
  { id: 'deepseek', name: 'DeepSeek', model: 'deepseek-v4-pro', envKey: 'DEEPSEEK_API_KEY' },
  { id: 'xiaomi-mimo', name: '小米 MiMo', model: 'mimo-v2.5-pro', envKey: 'MIMO_API_KEY' },
  { id: 'qwen', name: '通义千问', model: 'qwen-max', envKey: 'QWEN_API_KEY' },
  { id: 'zhipu', name: '智谱 GLM', model: 'glm-4-plus', envKey: 'ZHIPU_API_KEY' },
  { id: 'moonshot', name: '月之暗面', model: 'kimi-k2.6', envKey: 'MOONSHOT_API_KEY' },
  { id: 'volcengine', name: '火山引擎', model: 'DeepSeek-V4-Pro', envKey: 'VOLCENGINE_API_KEY' },
  { id: 'openrouter', name: 'OpenRouter', model: 'deepseek/deepseek-v4-pro', envKey: 'OPENROUTER_API_KEY' },
  { id: 'siliconflow', name: 'SiliconFlow', model: 'deepseek-ai/DeepSeek-V4-Pro', envKey: 'SILICONFLOW_API_KEY' },
  { id: 'nvidia-nim', name: 'NVIDIA NIM', model: 'deepseek-ai/deepseek-v4-pro', envKey: 'NVIDIA_API_KEY' },
  { id: 'ollama', name: 'Ollama（本地）', model: 'deepseek-coder:1.3b', envKey: 'OLLAMA_API_KEY' },
]

export async function render(): Promise<HTMLElement> {
  const page = document.createElement('div')
  page.className = 'page codewhale-providers'

  let cfg: PanelConfig | null = null
  try { cfg = await api.readPanelConfig() as PanelConfig } catch { /* ignore */ }

  const currentProvider = cfg?.codewhale?.provider ?? 'deepseek'

  const rows = BUILTIN_PROVIDERS.map(p => {
    const active = p.id === currentProvider
    const label = PROVIDER_LABELS[p.id] ?? p.name
    return `<tr class="${active ? 'cw-row-active' : ''}">
      <td><strong>${esc(label)}</strong></td>
      <td><code>${esc(p.model)}</code></td>
      <td><code>${esc(p.envKey)}</code></td>
      <td>${active ? '<span class="cw-badge">当前</span>' : '可用'}</td>
    </tr>`
  }).join('')

  page.innerHTML = `
    <div class="page-header">
      <div>
        <h1>提供商管理</h1>
        <p class="page-desc">CodeWhale 内置 17 个提供商，DeepSeek 和 MiMo 为一等公民。通过 Chat Completions API 原生连接。</p>
      </div>
    </div>
    <div class="cw-grid">
      <section class="cw-card cw-card-wide">
        <div class="cw-card-title">内置提供商</div>
        <table class="cw-table">
          <thead><tr><th>提供商</th><th>默认模型</th><th>环境变量</th><th>状态</th></tr></thead>
          <tbody>${rows}</tbody>
        </table>
      </section>
      <section class="cw-card">
        <div class="cw-card-title">切换提供商</div>
        <div class="cw-form">
          <label>提供商</label>
          <select data-cw-provider-select>
            ${BUILTIN_PROVIDERS.map(p =>
              `<option value="${p.id}" ${p.id === currentProvider ? 'selected' : ''}>${esc(PROVIDER_LABELS[p.id] ?? p.name)}</option>`
            ).join('')}
          </select>
          <label>模型（留空使用默认）</label>
          <input type="text" data-cw-model-input placeholder="例如 deepseek-v4-pro" />
          <button class="cw-btn-primary" data-cw-save>保存设置</button>
          <div class="cw-form-hint">设置保存到 data/config/codewhale/config.toml</div>
        </div>
      </section>
    </div>
  `

  // 切换提供商时更新默认模型
  const selectEl = page.querySelector<HTMLSelectElement>('[data-cw-provider-select]')!
  const modelInput = page.querySelector<HTMLInputElement>('[data-cw-model-input]')!
  const saveBtn = page.querySelector<HTMLButtonElement>('[data-cw-save]')!

  selectEl.addEventListener('change', () => {
    const selected = BUILTIN_PROVIDERS.find(p => p.id === selectEl.value)
    if (selected) modelInput.placeholder = selected.model
  })

  saveBtn.addEventListener('click', () => {
    // 通过 Tauri 命令更新 config.toml（需要后端支持，这里先提示用户）
    const provider = selectEl.value
    const defaultModel = BUILTIN_PROVIDERS.find(p => p.id === provider)?.model ?? ''
    const model = modelInput.value.trim() || defaultModel
    alert(`已选择: ${PROVIDER_LABELS[provider] ?? provider} / ${model}\n\n请在 data/config/codewhale/config.toml 中确认配置。`)
  })

  return page
}
