import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { t } from '../src/lib/i18n.ts'

const source = readFileSync(new URL('../src/engines/hermes/pages/channels.ts', import.meta.url), 'utf8')

function getChannelBlock(channelId) {
  const start = source.indexOf(`id: '${channelId}'`)
  assert.notEqual(start, -1, `未找到 Hermes ${channelId} 渠道入口`)
  const objectStart = source.lastIndexOf('{', start)
  let depth = 0
  for (let index = objectStart; index < source.length; index += 1) {
    const char = source[index]
    if (char === '{') depth += 1
    if (char === '}') depth -= 1
    if (depth === 0) return source.slice(objectStart, index + 1)
  }
  assert.fail(`未找到 Hermes ${channelId} 渠道入口结束位置`)
}

function extractEngineKeys(block) {
  return [...block.matchAll(/['"](engine\.[A-Za-z0-9_.-]+)['"]/g)].map(match => match[1])
}

test('Hermes bundled plugin 渠道页会暴露上游平台配置入口', () => {
  for (const channelId of ['teams', 'google_chat', 'irc', 'line', 'simplex']) {
    getChannelBlock(channelId)
  }
})

test('Hermes bundled plugin 渠道页会暴露运行时真实读取字段', () => {
  const expectedFields = {
    teams: ['clientId', 'clientSecret', 'tenantId', 'port', 'serviceUrl', 'allowFrom', 'allowAllUsers', 'homeChannel', 'homeChannelName'],
    google_chat: ['projectId', 'subscriptionName', 'serviceAccountJson', 'allowFrom', 'allowAllUsers', 'homeChannel', 'homeChannelName'],
    irc: ['server', 'port', 'nickname', 'channel', 'useTls', 'serverPassword', 'nickservPassword', 'allowFrom', 'allowAllUsers', 'homeChannel', 'homeChannelName'],
    line: ['channelAccessToken', 'channelSecret', 'port', 'host', 'publicUrl', 'allowFrom', 'allowedGroups', 'allowedRooms', 'allowAllUsers', 'homeChannel', 'slowResponseThreshold'],
    simplex: ['wsUrl', 'allowFrom', 'allowAllUsers', 'homeChannel', 'homeChannelName'],
  }

  for (const [channelId, fields] of Object.entries(expectedFields)) {
    const block = getChannelBlock(channelId)
    for (const field of fields) {
      assert.match(block, new RegExp(`key:\\s*'${field}'`), `${channelId} 缺少 ${field} 字段`)
    }
  }
})

test('Hermes bundled plugin 渠道页不会对新增平台渲染旧通用策略字段', () => {
  for (const channelId of ['teams', 'google_chat', 'irc', 'line', 'simplex']) {
    const block = getChannelBlock(channelId)
    assert.match(block, /policyFields:\s*\[/, `${channelId} 应显式声明访问策略字段`)
    assert.doesNotMatch(block, /key:\s*'dmPolicy'/, `${channelId} 不应显示旧私聊策略`)
    assert.doesNotMatch(block, /key:\s*'groupPolicy'/, `${channelId} 不应显示旧群组策略`)
    assert.doesNotMatch(block, /key:\s*'groupAllowFrom'/, `${channelId} 不应显示旧群组白名单`)
    assert.doesNotMatch(block, /key:\s*'requireMention'/, `${channelId} 不应显示旧 @Bot 唤醒开关`)
  }
})

test('Hermes bundled plugin 渠道页新增平台不会暴露翻译 key', () => {
  const keys = new Set()
  for (const channelId of ['teams', 'google_chat', 'irc', 'line', 'simplex']) {
    for (const key of extractEngineKeys(getChannelBlock(channelId))) keys.add(key)
  }

  assert.ok(keys.size > 0, '应能提取新增平台用到的 engine 翻译 key')
  for (const key of keys) {
    assert.notEqual(t(key), key, `${key} 缺少运行时翻译`)
  }
})

test('Hermes 渠道页会暴露平台级显示和进度策略入口', () => {
  for (const field of [
    'displayToolProgress',
    'displayShowReasoning',
    'displayToolPreviewLength',
    'displayStreaming',
    'displayCleanupProgress',
  ]) {
    assert.match(source, new RegExp(`key:\\s*'${field}'`), `缺少 ${field} 显示策略字段`)
  }

  for (const key of [
    'engine.hermesChannelDisplayBehavior',
    'engine.hermesChannelDisplayHint',
    'engine.hermesChannelDisplayToolProgress',
    'engine.hermesChannelDisplayToolProgressOff',
    'engine.hermesChannelDisplayToolProgressNew',
    'engine.hermesChannelDisplayToolProgressAll',
    'engine.hermesChannelDisplayToolProgressVerbose',
    'engine.hermesChannelDisplayStreaming',
    'engine.hermesChannelDisplayStreamingInherit',
    'engine.hermesChannelDisplayStreamingOn',
    'engine.hermesChannelDisplayStreamingOff',
    'engine.hermesChannelDisplayToolPreviewLength',
    'engine.hermesChannelDisplayShowReasoning',
    'engine.hermesChannelDisplayCleanupProgress',
  ]) {
    assert.notEqual(t(key), key, `${key} 缺少运行时翻译`)
  }
})
