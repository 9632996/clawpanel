import { api } from '../../../lib/tauri-api.ts'
import type { CodeWhaleStatus, PanelConfig, ProviderConfig } from '../types'

function esc(value: unknown): string {
  return String(value ?? '').replace(/[&<>"]/g, ch =>
    ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' } as Record<string, string>)[ch] ?? ch
  )
}

export async function render(): Promise<HTMLElement> {
  const page = document.createElement('div')
  page.className = 'page codewhale-settings'

  let status: CodeWhaleStatus | null = null
  let cfg: PanelConfig | null = null
  try {
    status = await api.codewhaleStatus() as CodeWhaleStatus
    cfg = await api.readPanelConfig() as PanelConfig
  } catch { /* ignore */ }

  const cw = cfg?.codewhale
  const provider = cw?.provider ?? 'deepseek'
  const model = cw?.model ?? 'deepseek-chat'
  const baseUrl = cw?.baseUrl ?? '-'
  const providers = cw?.providers ?? {}

  // 从 providers map 中获取当前提供商的 envKey
  const currentProviderCfg = providers[provider] as ProviderConfig | undefined
  const envKey = currentProviderCfg?.envKey ?? status?.envKey ?? '-'
  const envPresent = status?.envPresent ?? false

  page.innerHTML = `
    <div class="page-header">
      <div>
        <h1>CodeWhale 设置</h1>
        <p class="page-desc">引擎配置、API Key 状态、运行环境信息。</p>
      </div>
    </div>
    <div class="cw-grid">
      <section class="cw-card">
        <div class="cw-card-title">引擎信息</div>
        <div class="cw-kv-list">
          <div class="cw-kv"><span>版本</span><strong>${esc(status?.version ?? '-')}</strong></div>
          <div class="cw-kv"><span>二进制路径</span><code>${esc(status?.cliPath ?? '-')}</code></div>
          <div class="cw-kv"><span>TUI 路径</span><code>${esc(status?.tuiPath ?? '-')}</code></div>
          <div class="cw-kv"><span>配置目录</span><code>${esc(status?.codewhaleHome ?? '-')}</code></div>
          <div class="cw-kv"><span>配置文件</span><span>${status?.configExists ? '✓ 存在' : '✗ 缺失'}</span></div>
          <div class="cw-kv"><span>技能目录</span><code>${esc(status?.skillsPath ?? '-')}</code></div>
          <div class="cw-kv"><span>技能数量</span><strong>${status?.skillCount ?? 0}</strong></div>
        </div>
      </section>
      <section class="cw-card">
        <div class="cw-card-title">模型配置</div>
        <div class="cw-kv-list">
          <div class="cw-kv"><span>提供商</span><strong>${esc(provider)}</strong></div>
          <div class="cw-kv"><span>模型</span><strong>${esc(model)}</strong></div>
          <div class="cw-kv"><span>端点</span><code>${esc(baseUrl)}</code></div>
          <div class="cw-kv"><span>API Key 变量</span><code>${esc(envKey)}</code></div>
          <div class="cw-kv"><span>API Key 状态</span>
            <span class="${envPresent ? 'cw-text-ok' : 'cw-text-error'}">
              ${envPresent ? '✓ 已配置' : '✗ 未配置'}
            </span>
          </div>
        </div>
        ${!envPresent ? `
          <div class="cw-alert cw-alert-warning">
            <strong>API Key 未配置。</strong>
            请在 <code>data/config/model-credentials.env</code> 中添加对应的 API Key，
            或在系统环境变量中设置 <code>${esc(envKey)}</code>。
          </div>
        ` : ''}
      </section>
      <section class="cw-card cw-card-wide">
        <div class="cw-card-title">配置文件位置</div>
        <div class="cw-kv-list">
          <div class="cw-kv"><span>CodeWhale 配置</span><code>data/config/codewhale/config.toml</code></div>
          <div class="cw-kv"><span>模型凭证</span><code>data/config/model-credentials.env</code></div>
          <div class="cw-kv"><span>技能目录</span><code>data/config/codewhale/skills/</code></div>
        </div>
        <div class="cw-hint">
          编辑 <code>config.toml</code> 可切换默认提供商和模型。
          编辑 <code>model-credentials.env</code> 可配置 API Key。
        </div>
      </section>
    </div>
  `

  return page
}
