import test from 'node:test'
import assert from 'node:assert/strict'

import {
  formatRuntimeAge,
  getChannelRuntimeSummary,
  normalizeChannelRuntimeStatus,
} from '../src/lib/channel-runtime.ts'

test('normalizeChannelRuntimeStatus marks missing RPC result as unsupported', () => {
  const status = normalizeChannelRuntimeStatus(null)

  assert.equal(status.supported, false)
  assert.deepEqual(status.channelOrder, [])
  assert.deepEqual(status.channels, {})
})

test('getChannelRuntimeSummary preserves normalized unsupported status', () => {
  const status = normalizeChannelRuntimeStatus(null)
  const summary = getChannelRuntimeSummary(status, 'telegram', 'Telegram')

  assert.equal(summary.supported, false)
  assert.equal(summary.state, 'unsupported')
  assert.equal(summary.label, 'Telegram')
  assert.deepEqual(summary.accounts, [])
})

test('getChannelRuntimeSummary prefers account errors over connected state', () => {
  const status = normalizeChannelRuntimeStatus({
    ts: 1000,
    channelOrder: ['telegram'],
    channelLabels: { telegram: 'Telegram' },
    channelAccounts: {
      telegram: [
        {
          accountId: 'bot-a',
          configured: true,
          enabled: true,
          running: true,
          connected: true,
          lastError: '401 Unauthorized',
        },
      ],
    },
    channelDefaultAccountId: { telegram: 'bot-a' },
    channels: {},
  })

  const summary = getChannelRuntimeSummary(status, 'telegram')

  assert.equal(summary.supported, true)
  assert.equal(summary.state, 'error')
  assert.equal(summary.label, 'Telegram')
  assert.equal(summary.defaultAccountId, 'bot-a')
  assert.equal(summary.accounts[0].state, 'error')
  assert.equal(summary.accounts[0].lastError, '401 Unauthorized')
})

test('getChannelRuntimeSummary counts account states and preserves unknown channel fields', () => {
  const status = normalizeChannelRuntimeStatus({
    ts: 2000,
    partial: true,
    warnings: ['probe timeout'],
    channelOrder: ['slack'],
    channelLabels: { slack: 'Slack' },
    channelAccounts: {
      slack: [
        { accountId: 'team-a', configured: true, running: true },
        { accountId: 'team-b', configured: true, connected: true, audit: { messages: 3 } },
        { accountId: 'team-c', enabled: false, configured: true },
      ],
    },
    channelDefaultAccountId: { slack: 'team-a' },
    channels: { slack: { custom: 'value' } },
  })

  const summary = getChannelRuntimeSummary(status, 'slack')

  assert.equal(status.supported, true)
  assert.equal(status.partial, true)
  assert.deepEqual(status.warnings, ['probe timeout'])
  assert.deepEqual(status.channels.slack, { custom: 'value' })
  assert.equal(summary.state, 'connected')
  assert.equal(summary.counts.connected, 1)
  assert.equal(summary.counts.running, 1)
  assert.equal(summary.counts.disabled, 1)
})

test('formatRuntimeAge returns compact relative time labels', () => {
  assert.equal(formatRuntimeAge(1_000, 61_000), '1 分钟前')
  assert.equal(formatRuntimeAge(1_000, 3_601_000), '1 小时前')
  assert.equal(formatRuntimeAge(1_000, 172_801_000), '2 天前')
})
