import test from 'node:test'
import assert from 'node:assert/strict'

import {
  buildHermesLoggingConfigValues,
  mergeHermesLoggingConfig,
} from '../scripts/dev-api.js'

test('Hermes 运行日志配置读取会提供上游默认值', () => {
  const values = buildHermesLoggingConfigValues({})

  assert.deepEqual(values, {
    loggingLevel: 'INFO',
    loggingMaxSizeMb: 5,
    loggingBackupCount: 3,
    loggingMemoryMonitorEnabled: true,
    loggingMemoryMonitorIntervalSeconds: 300,
  })
})

test('Hermes 运行日志配置读取会回显 YAML 字段', () => {
  const values = buildHermesLoggingConfigValues({
    logging: {
      level: 'DEBUG',
      max_size_mb: 12,
      backup_count: 7,
      memory_monitor: {
        enabled: false,
        interval_seconds: 120,
      },
    },
  })

  assert.equal(values.loggingLevel, 'DEBUG')
  assert.equal(values.loggingMaxSizeMb, 12)
  assert.equal(values.loggingBackupCount, 7)
  assert.equal(values.loggingMemoryMonitorEnabled, false)
  assert.equal(values.loggingMemoryMonitorIntervalSeconds, 120)
})

test('Hermes 运行日志配置保存会保留未知字段并写入 logging', () => {
  const next = mergeHermesLoggingConfig({
    logging: {
      level: 'INFO',
      custom_flag: 'keep-logging',
      memory_monitor: {
        custom_flag: 'keep-memory-monitor',
      },
    },
    cron: { wrap_response: true },
    streaming: { enabled: true },
  }, {
    loggingLevel: 'WARNING',
    loggingMaxSizeMb: '20',
    loggingBackupCount: '5',
    loggingMemoryMonitorEnabled: true,
    loggingMemoryMonitorIntervalSeconds: '180',
  })

  assert.deepEqual(next.cron, { wrap_response: true })
  assert.deepEqual(next.streaming, { enabled: true })
  assert.equal(next.logging.level, 'WARNING')
  assert.equal(next.logging.max_size_mb, 20)
  assert.equal(next.logging.backup_count, 5)
  assert.equal(next.logging.memory_monitor.enabled, true)
  assert.equal(next.logging.memory_monitor.interval_seconds, 180)
  assert.equal(next.logging.custom_flag, 'keep-logging')
  assert.equal(next.logging.memory_monitor.custom_flag, 'keep-memory-monitor')
})

test('Hermes 运行日志配置保存会拒绝非法级别和越界值', () => {
  assert.throws(
    () => mergeHermesLoggingConfig({}, { loggingLevel: 'TRACE' }),
    /logging\.level/,
  )
  assert.throws(
    () => mergeHermesLoggingConfig({}, { loggingMaxSizeMb: '0' }),
    /logging\.max_size_mb/,
  )
  assert.throws(
    () => mergeHermesLoggingConfig({}, { loggingBackupCount: '-1' }),
    /logging\.backup_count/,
  )
  assert.throws(
    () => mergeHermesLoggingConfig({}, { loggingMemoryMonitorIntervalSeconds: '0' }),
    /logging\.memory_monitor\.interval_seconds/,
  )
})
