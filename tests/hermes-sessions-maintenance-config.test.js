import test from 'node:test'
import assert from 'node:assert/strict'

import {
  buildHermesSessionsMaintenanceConfigValues,
  mergeHermesSessionsMaintenanceConfig,
} from '../scripts/dev-api.js'

test('Hermes 会话维护配置读取会提供上游默认值', () => {
  const values = buildHermesSessionsMaintenanceConfigValues({})

  assert.deepEqual(values, {
    sessionsAutoPrune: false,
    sessionsRetentionDays: 90,
    sessionsVacuumAfterPrune: true,
    sessionsMinIntervalHours: 24,
    sessionsWriteJsonSnapshots: false,
  })
})

test('Hermes 会话维护配置读取会回显 YAML 字段', () => {
  const values = buildHermesSessionsMaintenanceConfigValues({
    sessions: {
      auto_prune: true,
      retention_days: 14,
      vacuum_after_prune: false,
      min_interval_hours: 6,
      write_json_snapshots: true,
    },
  })

  assert.equal(values.sessionsAutoPrune, true)
  assert.equal(values.sessionsRetentionDays, 14)
  assert.equal(values.sessionsVacuumAfterPrune, false)
  assert.equal(values.sessionsMinIntervalHours, 6)
  assert.equal(values.sessionsWriteJsonSnapshots, true)
})

test('Hermes 会话维护配置保存会保留未知字段并写入 sessions', () => {
  const next = mergeHermesSessionsMaintenanceConfig({
    sessions: {
      auto_prune: false,
      custom_flag: 'keep-sessions',
    },
    model: { provider: 'anthropic' },
    streaming: { enabled: true },
  }, {
    sessionsAutoPrune: true,
    sessionsRetentionDays: '30',
    sessionsVacuumAfterPrune: false,
    sessionsMinIntervalHours: '12',
    sessionsWriteJsonSnapshots: true,
  })

  assert.deepEqual(next.model, { provider: 'anthropic' })
  assert.deepEqual(next.streaming, { enabled: true })
  assert.equal(next.sessions.auto_prune, true)
  assert.equal(next.sessions.retention_days, 30)
  assert.equal(next.sessions.vacuum_after_prune, false)
  assert.equal(next.sessions.min_interval_hours, 12)
  assert.equal(next.sessions.write_json_snapshots, true)
  assert.equal(next.sessions.custom_flag, 'keep-sessions')
})

test('Hermes 会话维护配置保存会拒绝越界值', () => {
  assert.throws(
    () => mergeHermesSessionsMaintenanceConfig({}, { sessionsRetentionDays: '0' }),
    /sessions\.retention_days/,
  )
  assert.throws(
    () => mergeHermesSessionsMaintenanceConfig({}, { sessionsRetentionDays: '36501' }),
    /sessions\.retention_days/,
  )
  assert.throws(
    () => mergeHermesSessionsMaintenanceConfig({}, { sessionsMinIntervalHours: '-1' }),
    /sessions\.min_interval_hours/,
  )
  assert.throws(
    () => mergeHermesSessionsMaintenanceConfig({}, { sessionsMinIntervalHours: '87601' }),
    /sessions\.min_interval_hours/,
  )
})
