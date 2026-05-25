import test from 'node:test'
import assert from 'node:assert/strict'

import {
  buildHermesSttConfigValues,
  mergeHermesSttConfig,
} from '../scripts/dev-api.js'

test('Hermes STT 配置读取会提供上游默认值', () => {
  const values = buildHermesSttConfigValues({})

  assert.deepEqual(values, {
    sttEnabled: true,
    sttProvider: 'auto',
    sttLocalModel: 'base',
    sttLocalLanguage: '',
    sttOpenaiModel: 'whisper-1',
    sttMistralModel: 'voxtral-mini-latest',
  })
})

test('Hermes STT 配置读取会回显语音转写模型字段', () => {
  const values = buildHermesSttConfigValues({
    stt: {
      enabled: false,
      provider: 'openai',
      local: {
        model: 'small',
        language: 'zh',
      },
      openai: {
        model: 'gpt-4o-mini-transcribe',
      },
      mistral: {
        model: 'voxtral-mini-2602',
      },
    },
  })

  assert.equal(values.sttEnabled, false)
  assert.equal(values.sttProvider, 'openai')
  assert.equal(values.sttLocalModel, 'small')
  assert.equal(values.sttLocalLanguage, 'zh')
  assert.equal(values.sttOpenaiModel, 'gpt-4o-mini-transcribe')
  assert.equal(values.sttMistralModel, 'voxtral-mini-2602')
})

test('Hermes STT 配置保存会保留未知字段并写入上游结构', () => {
  const next = mergeHermesSttConfig({
    model: { provider: 'anthropic' },
    stt: {
      enabled: true,
      custom_flag: 'keep-stt',
      local: {
        model: 'base',
        custom_flag: 'keep-local',
      },
    },
    memory: { memory_enabled: true },
  }, {
    sttEnabled: false,
    sttProvider: 'openai',
    sttLocalModel: 'small',
    sttLocalLanguage: 'zh',
    sttOpenaiModel: 'gpt-4o-mini-transcribe',
    sttMistralModel: 'voxtral-mini-2602',
  })

  assert.deepEqual(next.model, { provider: 'anthropic' })
  assert.deepEqual(next.memory, { memory_enabled: true })
  assert.equal(next.stt.enabled, false)
  assert.equal(next.stt.provider, 'openai')
  assert.equal(next.stt.local.model, 'small')
  assert.equal(next.stt.local.language, 'zh')
  assert.equal(next.stt.openai.model, 'gpt-4o-mini-transcribe')
  assert.equal(next.stt.mistral.model, 'voxtral-mini-2602')
  assert.equal(next.stt.custom_flag, 'keep-stt')
  assert.equal(next.stt.local.custom_flag, 'keep-local')
})

test('Hermes STT 配置保存会拒绝非法枚举和语言标签', () => {
  assert.throws(
    () => mergeHermesSttConfig({}, { sttProvider: 'bad' }),
    /stt\.provider/,
  )
  assert.throws(
    () => mergeHermesSttConfig({}, { sttLocalModel: 'giant' }),
    /stt\.local\.model/,
  )
  assert.throws(
    () => mergeHermesSttConfig({}, { sttOpenaiModel: 'gpt-4.1' }),
    /stt\.openai\.model/,
  )
  assert.throws(
    () => mergeHermesSttConfig({}, { sttMistralModel: 'voxtral-large' }),
    /stt\.mistral\.model/,
  )
  assert.throws(
    () => mergeHermesSttConfig({}, { sttLocalLanguage: '中文' }),
    /stt\.local\.language/,
  )
})
