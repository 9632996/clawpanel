import { api } from '../../../lib/tauri-api.ts'
import type { CodeWhaleStatus, CodeWhaleRunResult, PanelConfig } from '../types'

interface ChatMessage {
  role: 'user' | 'assistant'
  content: string
  model?: string
  timestamp: number
}

const _messages: ChatMessage[] = []
let _isRunning = false

function esc(value: unknown): string {
  return String(value ?? '').replace(/[&<>"]/g, ch =>
    ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' } as Record<string, string>)[ch] ?? ch
  )
}

function renderMarkdown(text: string): string {
  return esc(text)
    .replace(/```(\w*)\n([\s\S]*?)```/g, '<pre class="cw-code-block"><code>$2</code></pre>')
    .replace(/`([^`]+)`/g, '<code class="cw-inline-code">$1</code>')
    .replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>')
    .replace(/\n/g, '<br>')
}

function renderMessage(msg: ChatMessage): string {
  const isUser = msg.role === 'user'
  const cls = isUser ? 'cw-msg-user' : 'cw-msg-agent'
  const label = isUser ? '你' : 'CodeWhale'
  const body = isUser
    ? `<div class="cw-msg-text">${esc(msg.content)}</div>`
    : `<div class="cw-msg-text">${renderMarkdown(msg.content)}</div>`
  const meta = msg.model ? `<div class="cw-msg-meta">${esc(msg.model)}</div>` : ''
  return `<div class="cw-msg ${cls}">
    <div class="cw-msg-header"><strong>${label}</strong>${meta}</div>
    ${body}
  </div>`
}

function refreshMessages(container: HTMLElement): void {
  if (_messages.length === 0) {
    container.innerHTML = '<div class="cw-empty">输入编码问题开始对话。支持代码审查、重构、测试生成等技能。</div>'
    return
  }
  container.innerHTML = _messages.map(renderMessage).join('')
  container.scrollTop = container.scrollHeight
}

function buildPromptWithContext(userInput: string): string {
  // CodeWhale exec 模式是单次调用，需要把对话历史拼入 prompt
  if (_messages.length <= 1) return userInput

  let prompt = '以下是对话历史，请基于上下文回答最后一个问题：\n'
  for (const msg of _messages) {
    const prefix = msg.role === 'user' ? 'User' : 'Assistant'
    prompt += `\n${prefix}: ${msg.content}\n`
  }
  prompt += `\nUser: ${userInput}`
  return prompt
}

export async function render(): Promise<HTMLElement> {
  const page = document.createElement('div')
  page.className = 'page codewhale-chat'

  // 获取状态
  let status: CodeWhaleStatus | null = null
  try {
    status = await api.codewhaleStatus() as CodeWhaleStatus
  } catch { /* ignore */ }

  const statusText = status?.ready
    ? `就绪 · ${status.version ?? 'v0.8'} · ${status.skillCount} 技能`
    : '未就绪'
  const statusClass = status?.ready ? 'cw-status-ready' : 'cw-status-error'

  page.innerHTML = `
    <div class="page-header">
      <div class="cw-header-row">
        <div>
          <h1>编码对话</h1>
          <p class="page-desc">与 CodeWhale 编码智能体对话，支持 DeepSeek/MiMo 等国内模型。</p>
        </div>
        <div class="cw-header-actions">
          <span class="cw-status-badge ${statusClass}">${esc(statusText)}</span>
          <button class="cw-btn-sm" data-cw-clear title="清空对话">清空</button>
        </div>
      </div>
    </div>
    <div class="cw-chat-layout">
      <div class="cw-messages" data-cw-messages></div>
      <div class="cw-input-bar">
        <textarea placeholder="输入编码问题... (Enter 发送，Shift+Enter 换行)" rows="3" data-cw-input></textarea>
        <div class="cw-input-actions">
          <button class="cw-btn-primary" data-cw-send>发送</button>
          <button class="cw-btn-danger" data-cw-stop style="display:none">停止</button>
        </div>
      </div>
    </div>
  `

  const messagesEl = page.querySelector<HTMLElement>('[data-cw-messages]')!
  const inputEl = page.querySelector<HTMLTextAreaElement>('[data-cw-input]')!
  const sendBtn = page.querySelector<HTMLButtonElement>('[data-cw-send]')!
  const stopBtn = page.querySelector<HTMLButtonElement>('[data-cw-stop]')!
  const clearBtn = page.querySelector<HTMLButtonElement>('[data-cw-clear]')!

  refreshMessages(messagesEl)

  async function sendMessage(): Promise<void> {
    const text = inputEl.value.trim()
    if (!text || _isRunning) return

    _messages.push({ role: 'user', content: text, timestamp: Date.now() })
    inputEl.value = ''
    refreshMessages(messagesEl)

    _isRunning = true
    sendBtn.disabled = true
    stopBtn.style.display = ''
    sendBtn.textContent = '执行中...'

    try {
      const prompt = buildPromptWithContext(text)
      const result = await api.codewhaleRunOnce(prompt) as CodeWhaleRunResult
      const raw = result.stdout || result.stderr || '无输出'
      // 清理 ANSI 转义码
      const clean = raw.replace(/\x1b\[[0-9;]*[a-zA-Z]/g, '').trim()
      _messages.push({
        role: 'assistant',
        content: clean || '（空响应）',
        model: 'CodeWhale',
        timestamp: Date.now(),
      })
    } catch (err: unknown) {
      _messages.push({
        role: 'assistant',
        content: `错误: ${err instanceof Error ? err.message : String(err)}`,
        model: 'CodeWhale',
        timestamp: Date.now(),
      })
    } finally {
      _isRunning = false
      sendBtn.disabled = false
      sendBtn.textContent = '发送'
      stopBtn.style.display = 'none'
      refreshMessages(messagesEl)
      inputEl.focus()
    }
  }

  sendBtn.addEventListener('click', () => { void sendMessage() })

  stopBtn.addEventListener('click', () => {
    _isRunning = false
    sendBtn.disabled = false
    sendBtn.textContent = '发送'
    stopBtn.style.display = 'none'
  })

  clearBtn.addEventListener('click', () => {
    _messages.length = 0
    refreshMessages(messagesEl)
  })

  inputEl.addEventListener('keydown', (e: KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      void sendMessage()
    }
  })

  inputEl.focus()
  return page
}
