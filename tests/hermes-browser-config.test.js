import test from 'node:test'
import assert from 'node:assert/strict'

import {
  buildHermesBrowserConfigValues,
  mergeHermesBrowserConfig,
} from '../scripts/dev-api.js'

test('Hermes 浏览器配置读取会提供上游默认值', () => {
  const values = buildHermesBrowserConfigValues({})

  assert.deepEqual(values, {
    browserInactivityTimeout: 120,
    browserCommandTimeout: 30,
    browserRecordSessions: false,
    browserEngine: 'auto',
  })
})

test('Hermes 浏览器配置读取会回显 YAML 字段', () => {
  const values = buildHermesBrowserConfigValues({
    browser: {
      inactivity_timeout: 300,
      command_timeout: 45,
      record_sessions: true,
      engine: 'lightpanda',
    },
  })

  assert.equal(values.browserInactivityTimeout, 300)
  assert.equal(values.browserCommandTimeout, 45)
  assert.equal(values.browserRecordSessions, true)
  assert.equal(values.browserEngine, 'lightpanda')
})

test('Hermes 浏览器配置保存会保留未知字段并写入上游结构', () => {
  const next = mergeHermesBrowserConfig({
    model: { provider: 'anthropic' },
    browser: {
      inactivity_timeout: 120,
      command_timeout: 30,
      record_sessions: false,
      engine: 'auto',
      cdp_url: 'ws://127.0.0.1:9222/devtools/browser/demo',
      camofox: { managed_persistence: true },
      custom_flag: 'keep-browser',
    },
    streaming: { enabled: true },
  }, {
    browserInactivityTimeout: '180',
    browserCommandTimeout: '60',
    browserRecordSessions: true,
    browserEngine: 'chrome',
  })

  assert.deepEqual(next.model, { provider: 'anthropic' })
  assert.deepEqual(next.streaming, { enabled: true })
  assert.equal(next.browser.inactivity_timeout, 180)
  assert.equal(next.browser.command_timeout, 60)
  assert.equal(next.browser.record_sessions, true)
  assert.equal(next.browser.engine, 'chrome')
  assert.equal(next.browser.cdp_url, 'ws://127.0.0.1:9222/devtools/browser/demo')
  assert.deepEqual(next.browser.camofox, { managed_persistence: true })
  assert.equal(next.browser.custom_flag, 'keep-browser')
})

test('Hermes 浏览器配置保存会拒绝非法引擎和越界值', () => {
  assert.throws(
    () => mergeHermesBrowserConfig({}, { browserEngine: 'firefox' }),
    /browser\.engine/,
  )
  assert.throws(
    () => mergeHermesBrowserConfig({}, { browserInactivityTimeout: '0' }),
    /browser\.inactivity_timeout/,
  )
  assert.throws(
    () => mergeHermesBrowserConfig({}, { browserCommandTimeout: '4' }),
    /browser\.command_timeout/,
  )
})
