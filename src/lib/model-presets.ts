// @ts-nocheck
/**
 * 共享模型预设配置
 * models.js 和 assistant.js 共用，只需维护一套数据
 */
import { ZHIZHUA_MODEL_BASE_URL, ZHIZHUA_SERVICE_URL } from './service-urls.ts'

// API 接口类型选项
export const API_TYPES = [
  { value: 'openai-completions', label: 'OpenAI Chat Completions (最常用)' },
  { value: 'anthropic-messages', label: 'Anthropic Messages' },
  { value: 'openai-responses', label: 'OpenAI Responses' },
  { value: 'openai-codex-responses', label: 'OpenAI Codex Responses' },
  { value: 'google-generative-ai', label: 'Google Gemini' },
  { value: 'github-copilot', label: 'GitHub Copilot' },
  { value: 'bedrock-converse-stream', label: 'AWS Bedrock' },
  { value: 'ollama', label: 'Ollama 本地模型' },
]

// 服务商快捷预设
export const PROVIDER_PRESETS = [
  { key: 'aizuopin', label: 'AI作品模型服务', badge: '推荐', baseUrl: ZHIZHUA_MODEL_BASE_URL, api: 'openai-completions', site: ZHIZHUA_SERVICE_URL, desc: '智爪工具入口内置的模型服务，支持构建器注入客户授权额度' },
  { key: 'shengsuanyun', label: '胜算云', baseUrl: 'https://router.shengsuanyun.com/api/v1', api: 'openai-completions', site: 'https://www.shengsuanyun.com/?from=CH_4BVI0BM2', desc: '国内知名 AI 模型聚合平台，支持多种主流模型' },
  { key: 'siliconflow', label: '硅基流动', baseUrl: 'https://api.siliconflow.cn/v1', api: 'openai-completions', site: 'https://cloud.siliconflow.cn/i/PFrw2an5', desc: '高性价比推理平台，支持 DeepSeek、Qwen 等开源模型' },
  { key: 'volcengine', label: '火山引擎', baseUrl: 'https://ark.cn-beijing.volces.com/api/v3', api: 'openai-completions', site: 'https://volcengine.com/L/Ph1OP5I3_GY', desc: '字节跳动旗下云平台，支持豆包等模型' },
  { key: 'aliyun', label: '阿里云百炼', baseUrl: 'https://dashscope.aliyuncs.com/compatible-mode/v1', api: 'openai-completions', site: 'https://www.aliyun.com/benefit/ai/aistar?userCode=keahn2zr&clubBiz=subTask..12435175..10263..', desc: '阿里云 AI 大模型平台，支持通义千问全系列' },
  { key: 'zhipu', label: '智谱 AI', baseUrl: 'https://open.bigmodel.cn/api/paas/v4', api: 'openai-completions', site: 'https://www.bigmodel.cn/glm-coding?ic=3F6F9XYKTS', desc: '国产大模型领军企业，支持 GLM-4 全系列' },
  { key: 'minimax', label: 'MiniMax', baseUrl: 'https://api.minimax.io/v1', api: 'openai-completions', site: 'https://platform.minimaxi.com/user-center/basic-information/interface-key', desc: '国产多模态大模型，支持 MiniMax-M2.7 / M2.5 系列' },
  { key: 'moonshot', label: 'Moonshot / Kimi', baseUrl: 'https://api.moonshot.ai/v1', api: 'openai-completions', site: 'https://platform.moonshot.ai/console/api-keys', desc: 'Kimi 大模型平台，支持超长上下文' },
  { key: 'openai', label: 'OpenAI 官方', baseUrl: 'https://api.openai.com/v1', api: 'openai-completions', site: 'https://platform.openai.com/api-keys' },
  { key: 'anthropic', label: 'Anthropic 官方', baseUrl: 'https://api.anthropic.com/v1', api: 'anthropic-messages', site: 'https://console.anthropic.com/settings/keys' },
  { key: 'deepseek', label: 'DeepSeek', baseUrl: 'https://api.deepseek.com/v1', api: 'openai-completions', site: 'https://platform.deepseek.com/api_keys' },
  { key: 'google', label: 'Google Gemini', baseUrl: 'https://generativelanguage.googleapis.com/v1beta', api: 'google-generative-ai', site: 'https://aistudio.google.com/app/apikey' },
  { key: 'xai', label: 'xAI (Grok)', baseUrl: 'https://api.x.ai/v1', api: 'openai-completions', site: 'https://console.x.ai/', desc: 'Elon Musk 旗下 AI，支持 Grok 系列模型' },
  { key: 'groq', label: 'Groq', baseUrl: 'https://api.groq.com/openai/v1', api: 'openai-completions', site: 'https://console.groq.com/keys', desc: '超快推理平台，支持 Llama、Mixtral 等开源模型' },
  { key: 'openrouter', label: 'OpenRouter', baseUrl: 'https://openrouter.ai/api/v1', api: 'openai-completions', site: 'https://openrouter.ai/keys', desc: '模型聚合路由，一个 Key 访问所有主流模型' },
  { key: 'nvidia', label: 'NVIDIA NIM', baseUrl: 'https://integrate.api.nvidia.com/v1', api: 'openai-completions', site: 'https://build.nvidia.com/models', desc: '英伟达推理平台，支持 Llama、Mistral 等模型' },
  { key: 'ollama', label: 'Ollama (本地)', baseUrl: 'http://127.0.0.1:11434/v1', api: 'openai-completions', site: 'https://ollama.com/' },
]

// AI作品模型服务配置
export const QTCOOL = {
  baseUrl: ZHIZHUA_MODEL_BASE_URL,
  defaultKey: '',
  site: ZHIZHUA_SERVICE_URL,
  checkinUrl: ZHIZHUA_SERVICE_URL,
  usageUrl: ZHIZHUA_SERVICE_URL,
  providerKey: 'aizuopin',
  brandName: 'AI作品模型服务',
  api: 'openai-completions',
  models: []  // 始终从 API 动态获取最新模型列表
}

// 胜算云推广配置
export const SHENGSUANYUN = {
  baseUrl: 'https://router.shengsuanyun.com/api/v1',
  site: 'https://www.shengsuanyun.com/?from=CH_4BVI0BM2',
  providerKey: 'shengsuanyun',
  brandName: '胜算云',
  api: 'openai-completions',
}

// 常用模型预设（按服务商分组）
export const MODEL_PRESETS = {
  openai: [
    { id: 'gpt-4o', name: 'GPT-4o', contextWindow: 128000 },
    { id: 'gpt-4o-mini', name: 'GPT-4o Mini', contextWindow: 128000 },
    { id: 'o3-mini', name: 'o3 Mini', contextWindow: 200000, reasoning: true },
  ],
  anthropic: [
    { id: 'claude-sonnet-4-5-20250514', name: 'Claude Sonnet 4.5', contextWindow: 200000 },
    { id: 'claude-haiku-3-5-20241022', name: 'Claude Haiku 3.5', contextWindow: 200000 },
  ],
  deepseek: [
    { id: 'deepseek-chat', name: 'DeepSeek V3', contextWindow: 64000 },
    { id: 'deepseek-reasoner', name: 'DeepSeek R1', contextWindow: 64000, reasoning: true },
  ],
  google: [
    { id: 'gemini-2.5-pro', name: 'Gemini 2.5 Pro', contextWindow: 1000000, reasoning: true },
    { id: 'gemini-2.5-flash', name: 'Gemini 2.5 Flash', contextWindow: 1000000 },
  ],
  minimax: [
    { id: 'MiniMax-M2.7', name: 'MiniMax M2.7', contextWindow: 1000000 },
    { id: 'MiniMax-M2.7-highspeed', name: 'MiniMax M2.7 Highspeed', contextWindow: 1000000 },
    { id: 'MiniMax-M2.5', name: 'MiniMax M2.5', contextWindow: 204000 },
    { id: 'MiniMax-M2.5-highspeed', name: 'MiniMax M2.5 Highspeed', contextWindow: 204000 },
  ],
  moonshot: [
    { id: 'kimi-k2.5', name: 'Kimi K2.5', contextWindow: 131072 },
    { id: 'kimi-k2', name: 'Kimi K2', contextWindow: 131072 },
    { id: 'kimi-latest', name: 'Kimi Latest', contextWindow: 131072 },
  ],
  xai: [
    { id: 'grok-4', name: 'Grok 4', contextWindow: 131072 },
    { id: 'grok-4-fast', name: 'Grok 4 Fast', contextWindow: 131072 },
  ],
  groq: [
    { id: 'llama-3.3-70b-versatile', name: 'Llama 3.3 70B', contextWindow: 32768 },
    { id: 'mixtral-8x7b-32768', name: 'Mixtral 8x7B', contextWindow: 32768 },
  ],
  ollama: [
    { id: 'qwen3:32b', name: 'Qwen 3 32B', contextWindow: 32768 },
    { id: 'llama3.3:70b', name: 'Llama 3.3 70B', contextWindow: 8192 },
    { id: 'deepseek-r1:32b', name: 'DeepSeek R1 32B', contextWindow: 32768, reasoning: true },
  ],
}

/**
 * 动态获取 QTCOOL 模型列表
 * @param {string} [apiKey] - 自定义密钥；未传时尝试从已有配置读取
 * @returns {Promise<Array<{id:string, name:string, contextWindow:number, reasoning?:boolean}>>}
 */
export async function fetchQtcoolModels(apiKey) {
  const typedKey = typeof apiKey === 'string' ? apiKey.trim() : ''
  const toModel = (id) => ({
    id, name: id, contextWindow: 128000,
    reasoning: id.includes('codex')
  })
  let lastError = null

  try {
    const { api } = await import('../lib/tauri-api.ts')
    const cfg = await api.readOpenclawConfig()
    const providers = cfg?.models?.providers || {}
    const provider = providers[QTCOOL.providerKey] || providers.aizuopin || providers.qtcool || null
    const baseUrl = provider?.baseUrl || QTCOOL.baseUrl
    const apiType = provider?.api || QTCOOL.api
    const key = typedKey || provider?.apiKey || QTCOOL.defaultKey || ''
    const ids = await api.listRemoteModels(baseUrl, key, apiType)
    if (Array.isArray(ids) && ids.length) {
      return ids.map(toModel).sort((a, b) => a.id.localeCompare(b.id))
    }
  } catch (err) {
    lastError = err
  }

  const fallbackKey = typedKey || QTCOOL.defaultKey
  if (!fallbackKey && lastError) {
    throw lastError
  }
  try {
    const key = fallbackKey
    const headers = key ? { 'Authorization': 'Bearer ' + key } : {}
    const resp = await fetch(QTCOOL.baseUrl + '/models', {
      headers,
      signal: AbortSignal.timeout(8000)
    })
    if (resp.ok) {
      const data = await resp.json()
      if (data.data && data.data.length) {
        return data.data.map(m => toModel(m.id)).sort((a, b) => a.id.localeCompare(b.id))
      }
    }
    if (!resp.ok) {
      lastError = lastError || new Error(`HTTP ${resp.status}`)
    }
  } catch (err) {
    lastError = lastError || err
  }
  if (lastError && !QTCOOL.models.length) {
    throw lastError
  }
  return QTCOOL.models
}
