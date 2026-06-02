import { api } from '../../../lib/tauri-api.js'

export async function render() {
  const page = document.createElement('div')
  page.className = 'page codex-dashboard'
  page.innerHTML = `
    <div class="page-header">
      <div>
        <h1>Codex 控制台</h1>
        <p class="page-desc">源码级内置的编码智能体，后续在本产品内适配国内兼容 OpenAI 的模型端点。</p>
      </div>
    </div>
    <div class="codex-grid">
      <section class="codex-panel">
        <div class="codex-panel-title">运行状态</div>
        <div class="codex-status-row">
          <span>源码</span>
          <strong data-codex-source>检测中</strong>
        </div>
        <div class="codex-status-row">
          <span>默认模型</span>
          <strong data-codex-model>gpt-5.4</strong>
        </div>
        <div class="codex-status-row">
          <span>默认端点</span>
          <strong data-codex-endpoint>https://ai.aizuopin.com/v1</strong>
        </div>
      </section>
      <section class="codex-panel">
        <div class="codex-panel-title">产品集成阶段</div>
        <div class="codex-stage-list">
          <div>已接入工作台三引擎切换</div>
          <div>已纳入 builder 目标目录发布</div>
          <div>待接入 Codex 后端执行和国内模型适配</div>
        </div>
      </section>
    </div>
  `

  try {
    const cfg = await api.readPanelConfig()
    page.querySelector('[data-codex-model]').textContent = cfg?.codex?.model || 'gpt-5.4'
    page.querySelector('[data-codex-endpoint]').textContent = cfg?.codex?.baseUrl || 'https://ai.aizuopin.com/v1'
    page.querySelector('[data-codex-source]').textContent = cfg?.codex?.sourceDir ? '已内置源码' : '等待构建器注入'
  } catch {
    page.querySelector('[data-codex-source]').textContent = '等待构建器注入'
  }

  return page
}
