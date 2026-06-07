// @ts-nocheck
import { api } from '../../../lib/tauri-api.ts'

function esc(value) {
  return String(value ?? '').replace(/[&<>"']/g, ch => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[ch]))
}

function display(value, fallback = '-') {
  const text = String(value ?? '').trim()
  return text || fallback
}

export async function render() {
  const page = document.createElement('div')
  page.className = 'page codex-native-app-page'
  page.innerHTML = `
    <div class="page-header codex-native-app-header">
      <div>
        <h1>原生 Codex App</h1>
        <p class="page-desc">智爪平台负责写入模型配置和 API Key，然后调起 Codex 官方原生应用。</p>
      </div>
      <button class="btn btn-primary" type="button" data-codex-launch>打开 Codex App</button>
    </div>

    <section class="codex-native-app-layout">
      <div class="codex-panel codex-native-app-primary">
        <div class="codex-panel-title">启动状态</div>
        <div class="codex-launch-state" data-codex-launch-state>
          <strong>准备中</strong>
          <span>正在读取便携版 Codex 配置</span>
        </div>
        <div class="codex-native-app-actions">
          <button class="btn btn-primary" type="button" data-codex-launch-secondary>打开原生 Codex App</button>
          <button class="btn btn-secondary" type="button" data-codex-refresh>刷新状态</button>
        </div>
      </div>

      <div class="codex-panel">
        <div class="codex-panel-title">当前模型配置</div>
        <div class="codex-status-row"><span>提供商</span><strong data-codex-provider>-</strong></div>
        <div class="codex-status-row"><span>模型</span><strong data-codex-model>-</strong></div>
        <div class="codex-status-row"><span>Base URL</span><strong data-codex-base>-</strong></div>
        <div class="codex-status-row"><span>API Key</span><strong data-codex-key>-</strong></div>
      </div>

      <div class="codex-panel codex-panel-wide">
        <div class="codex-panel-title">便携路径</div>
        <div class="codex-status-row"><span>Codex 二进制</span><strong data-codex-cli>-</strong></div>
        <div class="codex-status-row"><span>CODEX_HOME</span><strong data-codex-home>-</strong></div>
        <div class="codex-status-row"><span>工作目录</span><strong data-codex-workspace>-</strong></div>
      </div>
    </section>
  `

  const launchBtns = page.querySelectorAll('[data-codex-launch], [data-codex-launch-secondary]')
  const refreshBtn = page.querySelector('[data-codex-refresh]')
  const stateEl = page.querySelector('[data-codex-launch-state]')

  const setState = (tone, title, detail) => {
    stateEl.className = `codex-launch-state ${tone}`
    stateEl.innerHTML = `<strong>${esc(title)}</strong><span>${esc(detail)}</span>`
  }

  const setLaunchDisabled = disabled => {
    launchBtns.forEach(btn => { btn.disabled = disabled })
  }

  const loadStatus = async () => {
    setState('checking', '准备中', '正在读取便携版 Codex 配置')
    setLaunchDisabled(true)
    try {
      const status = await api.codexStatus()
      page.querySelector('[data-codex-provider]').textContent = display(status.provider, 'aizuopin')
      page.querySelector('[data-codex-model]').textContent = display(status.model, 'gpt-5.4')
      page.querySelector('[data-codex-base]').textContent = display(status.baseUrl)
      page.querySelector('[data-codex-key]').textContent = status.envPresent ? `${display(status.envKey, 'API Key')} 已注入` : `${display(status.envKey, 'API Key')} 未注入`
      page.querySelector('[data-codex-cli]').textContent = status.cliExists ? status.cliPath : `未部署: ${display(status.cliPath)}`
      page.querySelector('[data-codex-home]').textContent = display(status.codexHome)
      page.querySelector('[data-codex-workspace]').textContent = display(status.root ? `${status.root}\\data\\workspace\\main` : '')

      if (!status.cliExists) {
        setState('error', 'Codex 未部署', status.cliPath || '缺少 app/engines/codex/bin/codex')
        return
      }
      if (!status.configExists) {
        setState('error', 'Codex 配置缺失', status.configPath || '缺少 data/config/codex/config.toml')
        return
      }
      if (!status.envPresent) {
        setState('error', 'API Key 未注入', `${display(status.envKey, 'API Key')} 未在模型凭据中找到`)
        return
      }

      setState('ok', '可以启动', '将使用便携版 Codex 配置打开官方原生 Codex App')
      setLaunchDisabled(false)
    } catch (err) {
      setState('error', '状态读取失败', err?.message || String(err))
    }
  }

  const launch = async () => {
    setLaunchDisabled(true)
    setState('checking', '正在启动', '正在调起 Codex 官方原生应用')
    try {
      const result = await api.codexLaunchApp()
      setState('ok', '已发起启动', `工作目录：${result.workspace || '-'}；日志：${result.logs || '-'}`)
    } catch (err) {
      setState('error', '启动失败', err?.message || String(err))
    } finally {
      await loadStatus()
    }
  }

  launchBtns.forEach(btn => btn.addEventListener('click', launch))
  refreshBtn.addEventListener('click', loadStatus)
  await loadStatus()
  return page
}
