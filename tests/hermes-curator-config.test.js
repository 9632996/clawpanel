import test from 'node:test'
import assert from 'node:assert/strict'

import {
  buildHermesCuratorConfigValues,
  mergeHermesCuratorConfig,
} from '../scripts/dev-api.js'

test('Hermes Curator 配置读取会提供上游默认值', () => {
  const values = buildHermesCuratorConfigValues({})

  assert.deepEqual(values, {
    curatorEnabled: true,
    curatorIntervalHours: 168,
    curatorMinIdleHours: 2,
    curatorStaleAfterDays: 30,
    curatorArchiveAfterDays: 90,
    curatorBackupEnabled: true,
    curatorBackupKeep: 5,
  })
})

test('Hermes Curator 配置读取会回显 YAML 字段', () => {
  const values = buildHermesCuratorConfigValues({
    curator: {
      enabled: false,
      interval_hours: 24,
      min_idle_hours: 6,
      stale_after_days: 14,
      archive_after_days: 45,
      backup: {
        enabled: false,
        keep: 9,
      },
    },
  })

  assert.equal(values.curatorEnabled, false)
  assert.equal(values.curatorIntervalHours, 24)
  assert.equal(values.curatorMinIdleHours, 6)
  assert.equal(values.curatorStaleAfterDays, 14)
  assert.equal(values.curatorArchiveAfterDays, 45)
  assert.equal(values.curatorBackupEnabled, false)
  assert.equal(values.curatorBackupKeep, 9)
})

test('Hermes Curator 配置保存会保留未知字段并写入上游结构', () => {
  const next = mergeHermesCuratorConfig({
    curator: {
      enabled: true,
      backup: {
        enabled: true,
        custom_flag: 'keep-backup',
      },
      custom_flag: 'keep-curator',
    },
    skills: { external_dirs: ['~/.agents/skills'] },
    model: { provider: 'anthropic' },
  }, {
    curatorEnabled: false,
    curatorIntervalHours: '48',
    curatorMinIdleHours: '4',
    curatorStaleAfterDays: '21',
    curatorArchiveAfterDays: '60',
    curatorBackupEnabled: false,
    curatorBackupKeep: '3',
  })

  assert.deepEqual(next.skills, { external_dirs: ['~/.agents/skills'] })
  assert.deepEqual(next.model, { provider: 'anthropic' })
  assert.equal(next.curator.enabled, false)
  assert.equal(next.curator.interval_hours, 48)
  assert.equal(next.curator.min_idle_hours, 4)
  assert.equal(next.curator.stale_after_days, 21)
  assert.equal(next.curator.archive_after_days, 60)
  assert.equal(next.curator.backup.enabled, false)
  assert.equal(next.curator.backup.keep, 3)
  assert.equal(next.curator.backup.custom_flag, 'keep-backup')
  assert.equal(next.curator.custom_flag, 'keep-curator')
})

test('Hermes Curator 配置保存会拒绝越界和不一致保留期', () => {
  assert.throws(
    () => mergeHermesCuratorConfig({}, { curatorIntervalHours: '0' }),
    /curator\.interval_hours/,
  )
  assert.throws(
    () => mergeHermesCuratorConfig({}, { curatorMinIdleHours: '-1' }),
    /curator\.min_idle_hours/,
  )
  assert.throws(
    () => mergeHermesCuratorConfig({}, { curatorBackupKeep: '1001' }),
    /curator\.backup\.keep/,
  )
  assert.throws(
    () => mergeHermesCuratorConfig({}, {
      curatorStaleAfterDays: '90',
      curatorArchiveAfterDays: '30',
    }),
    /curator\.archive_after_days/,
  )
})
