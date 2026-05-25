import test from 'node:test'
import assert from 'node:assert/strict'

import {
  buildHermesModelConfigValues,
  mergeHermesModelConfig,
} from '../scripts/dev-api.js'

test('Hermes 基础模型配置读取会提供默认值并兼容 model.model', () => {
  assert.deepEqual(buildHermesModelConfigValues({}), {
    modelDefault: '',
    modelProvider: 'auto',
    modelBaseUrl: '',
    modelContextLength: '',
    modelMaxTokens: '',
  })

  const values = buildHermesModelConfigValues({
    model: {
      model: 'anthropic/claude-sonnet-4-6',
      provider: 'openrouter',
      base_url: 'https://openrouter.ai/api/v1',
      context_length: 131072,
      max_tokens: 8192,
    },
  })

  assert.deepEqual(values, {
    modelDefault: 'anthropic/claude-sonnet-4-6',
    modelProvider: 'openrouter',
    modelBaseUrl: 'https://openrouter.ai/api/v1',
    modelContextLength: '131072',
    modelMaxTokens: '8192',
  })
})

test('Hermes 基础模型配置保存会保留未知字段并写入 model.default/provider/base_url', () => {
  const next = mergeHermesModelConfig({
    model: {
      default: 'old-model',
      provider: 'auto',
      base_url: 'https://old.example/v1',
      auth_mode: 'env',
      context_length: 200000,
    },
    memory: { memory_enabled: true },
  }, {
    modelDefault: 'anthropic/claude-opus-4.6',
    modelProvider: 'openrouter',
    modelBaseUrl: 'https://openrouter.ai/api/v1',
    modelContextLength: '262144',
    modelMaxTokens: '16384',
  })

  assert.deepEqual(next.memory, { memory_enabled: true })
  assert.equal(next.model.default, 'anthropic/claude-opus-4.6')
  assert.equal(next.model.provider, 'openrouter')
  assert.equal(next.model.base_url, 'https://openrouter.ai/api/v1')
  assert.equal(next.model.context_length, 262144)
  assert.equal(next.model.max_tokens, 16384)
  assert.equal(next.model.auth_mode, 'env')
})

test('Hermes 基础模型配置保存空 base_url 会删除该字段但保留 model 其它字段', () => {
  const next = mergeHermesModelConfig({
    model: {
      default: 'old-model',
      provider: 'custom',
      base_url: 'https://old.example/v1',
      max_tokens: 8192,
    },
    display: { language: 'zh' },
  }, {
    modelDefault: 'google/gemini-3-flash-preview',
    modelProvider: 'auto',
    modelBaseUrl: '  ',
    modelContextLength: '',
    modelMaxTokens: ' ',
  })

  assert.equal(next.model.default, 'google/gemini-3-flash-preview')
  assert.equal(next.model.provider, 'auto')
  assert.equal(Object.hasOwn(next.model, 'base_url'), false)
  assert.equal(Object.hasOwn(next.model, 'context_length'), false)
  assert.equal(Object.hasOwn(next.model, 'max_tokens'), false)
  assert.deepEqual(next.display, { language: 'zh' })
})

test('Hermes 基础模型配置保存会拒绝空模型和字段类型错误', () => {
  assert.throws(
    () => mergeHermesModelConfig({}, { modelDefault: '  ', modelProvider: 'auto' }),
    /model\.default/,
  )
  assert.throws(
    () => mergeHermesModelConfig({}, { modelDefault: 'gpt-5', modelProvider: 123 }),
    /model\.provider/,
  )
  assert.throws(
    () => mergeHermesModelConfig({}, { modelDefault: 'gpt-5', modelProvider: 'auto', modelBaseUrl: 123 }),
    /model\.base_url/,
  )
  assert.throws(
    () => mergeHermesModelConfig({}, { modelDefault: 'gpt-5', modelProvider: 'auto', modelContextLength: '0' }),
    /model\.context_length/,
  )
  assert.throws(
    () => mergeHermesModelConfig({}, { modelDefault: 'gpt-5', modelProvider: 'auto', modelMaxTokens: '1.5' }),
    /model\.max_tokens/,
  )
})
