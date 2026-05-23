import test from 'node:test'
import assert from 'node:assert/strict'

import {
  buildHermesToolLoopGuardrailsConfigValues,
  mergeHermesToolLoopGuardrailsConfig,
} from '../scripts/dev-api.js'

test('Hermes 工具循环防护读取会提供上游默认值', () => {
  const values = buildHermesToolLoopGuardrailsConfigValues({})

  assert.deepEqual(values, {
    warningsEnabled: true,
    hardStopEnabled: false,
    warnExactFailure: 2,
    warnSameToolFailure: 3,
    warnNoProgress: 2,
    hardStopExactFailure: 5,
    hardStopSameToolFailure: 8,
    hardStopNoProgress: 5,
  })
})

test('Hermes 工具循环防护读取会回显嵌套阈值字段', () => {
  const values = buildHermesToolLoopGuardrailsConfigValues({
    tool_loop_guardrails: {
      warnings_enabled: false,
      hard_stop_enabled: true,
      warn_after: {
        exact_failure: 3,
        same_tool_failure: 4,
        idempotent_no_progress: 5,
      },
      hard_stop_after: {
        exact_failure: 6,
        same_tool_failure: 7,
        idempotent_no_progress: 8,
      },
    },
  })

  assert.equal(values.warningsEnabled, false)
  assert.equal(values.hardStopEnabled, true)
  assert.equal(values.warnExactFailure, 3)
  assert.equal(values.warnSameToolFailure, 4)
  assert.equal(values.warnNoProgress, 5)
  assert.equal(values.hardStopExactFailure, 6)
  assert.equal(values.hardStopSameToolFailure, 7)
  assert.equal(values.hardStopNoProgress, 8)
})

test('Hermes 工具循环防护保存会保留无关 YAML 并写入上游嵌套结构', () => {
  const next = mergeHermesToolLoopGuardrailsConfig({
    model: { provider: 'anthropic' },
    tool_loop_guardrails: {
      warnings_enabled: true,
      custom_flag: 'keep-me',
      warn_after: {
        exact_failure: 2,
        custom_warn: 99,
      },
    },
    streaming: { enabled: true },
  }, {
    warningsEnabled: false,
    hardStopEnabled: true,
    warnExactFailure: '3',
    warnSameToolFailure: '4',
    warnNoProgress: '5',
    hardStopExactFailure: '6',
    hardStopSameToolFailure: '7',
    hardStopNoProgress: '8',
  })

  assert.deepEqual(next.model, { provider: 'anthropic' })
  assert.deepEqual(next.streaming, { enabled: true })
  assert.equal(next.tool_loop_guardrails.warnings_enabled, false)
  assert.equal(next.tool_loop_guardrails.hard_stop_enabled, true)
  assert.equal(next.tool_loop_guardrails.custom_flag, 'keep-me')
  assert.equal(next.tool_loop_guardrails.warn_after.exact_failure, 3)
  assert.equal(next.tool_loop_guardrails.warn_after.same_tool_failure, 4)
  assert.equal(next.tool_loop_guardrails.warn_after.idempotent_no_progress, 5)
  assert.equal(next.tool_loop_guardrails.warn_after.custom_warn, 99)
  assert.equal(next.tool_loop_guardrails.hard_stop_after.exact_failure, 6)
  assert.equal(next.tool_loop_guardrails.hard_stop_after.same_tool_failure, 7)
  assert.equal(next.tool_loop_guardrails.hard_stop_after.idempotent_no_progress, 8)
})

test('Hermes 工具循环防护保存会拒绝越界阈值', () => {
  assert.throws(
    () => mergeHermesToolLoopGuardrailsConfig({}, { warnExactFailure: '0' }),
    /tool_loop_guardrails\.warn_after\.exact_failure/,
  )
  assert.throws(
    () => mergeHermesToolLoopGuardrailsConfig({}, { warnSameToolFailure: '101' }),
    /tool_loop_guardrails\.warn_after\.same_tool_failure/,
  )
  assert.throws(
    () => mergeHermesToolLoopGuardrailsConfig({}, { hardStopExactFailure: '0' }),
    /tool_loop_guardrails\.hard_stop_after\.exact_failure/,
  )
  assert.throws(
    () => mergeHermesToolLoopGuardrailsConfig({}, { hardStopNoProgress: '101' }),
    /tool_loop_guardrails\.hard_stop_after\.idempotent_no_progress/,
  )
})
