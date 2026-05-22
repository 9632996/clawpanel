import test from 'node:test'
import assert from 'node:assert/strict'

import {
  buildHermesChannelEnvUpdates,
  buildHermesChannelConfigValues,
  mergeHermesChannelConfig,
} from '../scripts/dev-api.js'

test('Hermes 渠道读取会从 platforms 平台配置生成稳定表单值', () => {
  const values = buildHermesChannelConfigValues({
    platforms: {
      telegram: {
        enabled: true,
        token: '123:token',
        extra: {
          dm_policy: 'pair',
          group_policy: 'allowlist',
          allow_from: ['1001', '1002'],
          require_mention: true,
        },
      },
    },
  })

  assert.equal(values.telegram.enabled, true)
  assert.equal(values.telegram.botToken, '123:token')
  assert.equal(values.telegram.dmPolicy, 'pair')
  assert.equal(values.telegram.groupPolicy, 'allowlist')
  assert.equal(values.telegram.allowFrom, '1001, 1002')
  assert.equal(values.telegram.requireMention, true)
})

test('Hermes 渠道读取会按运行时优先级合并 .env 凭证', () => {
  const values = buildHermesChannelConfigValues({
    platforms: {
      telegram: {
        enabled: true,
        token: 'yaml-token',
        extra: {
          allow_from: ['1001'],
        },
      },
      feishu: {
        enabled: true,
        extra: {
          app_id: 'yaml-app-id',
          app_secret: 'yaml-secret',
          domain: 'lark',
          connection_mode: 'webhook',
        },
      },
      dingtalk: {
        enabled: true,
        extra: {
          client_id: 'yaml-client-id',
          client_secret: 'yaml-client-secret',
          allowed_users: ['staff-1'],
          allowed_chats: ['cid-1'],
        },
      },
    },
  }, {
    TELEGRAM_BOT_TOKEN: 'env-token',
    FEISHU_APP_ID: 'env-app-id',
    FEISHU_APP_SECRET: 'env-secret',
    FEISHU_DOMAIN: 'feishu',
    FEISHU_CONNECTION_MODE: 'websocket',
    DINGTALK_CLIENT_ID: 'env-client-id',
    DINGTALK_CLIENT_SECRET: 'env-client-secret',
  })

  assert.equal(values.telegram.botToken, 'env-token')
  assert.equal(values.telegram.allowFrom, '1001')
  assert.equal(values.feishu.appId, 'env-app-id')
  assert.equal(values.feishu.appSecret, 'env-secret')
  assert.equal(values.feishu.domain, 'feishu')
  assert.equal(values.feishu.connectionMode, 'websocket')
  assert.equal(values.dingtalk.clientId, 'env-client-id')
  assert.equal(values.dingtalk.clientSecret, 'env-client-secret')
  assert.equal(values.dingtalk.allowFrom, 'staff-1')
  assert.equal(values.dingtalk.groupAllowFrom, 'cid-1')
})

test('Hermes 渠道保存会写入 Hermes 最新 platforms 配置并保留无关配置', () => {
  const next = mergeHermesChannelConfig({
    model: { provider: 'anthropic', default: 'claude-sonnet-4-6' },
    platforms: {
      telegram: {
        enabled: false,
        token: 'old',
        extra: {
          unknown_option: 'keep-me',
        },
      },
    },
  }, 'telegram', {
    enabled: true,
    botToken: '123:token',
    dmPolicy: 'pair',
    groupPolicy: 'allowlist',
    allowFrom: '1001, 1002',
    requireMention: true,
  })

  assert.deepEqual(next.model, { provider: 'anthropic', default: 'claude-sonnet-4-6' })
  assert.equal(next.platforms.telegram.enabled, true)
  assert.equal(next.platforms.telegram.token, undefined)
  assert.equal(next.platforms.telegram.extra.dm_policy, 'pair')
  assert.equal(next.platforms.telegram.extra.group_policy, 'allowlist')
  assert.deepEqual(next.platforms.telegram.extra.allow_from, ['1001', '1002'])
  assert.equal(next.platforms.telegram.extra.require_mention, true)
  assert.equal(next.platforms.telegram.extra.unknown_option, 'keep-me')
})

test('Hermes 飞书保存会补齐可运行默认项并使用 Hermes snake_case 字段', () => {
  const next = mergeHermesChannelConfig({}, 'feishu', {
    enabled: true,
    appId: 'cli_xxx',
    appSecret: 'secret',
    domain: '',
    connectionMode: '',
    webhookPath: '',
    reactionNotifications: '',
    typingIndicator: true,
    resolveSenderNames: true,
  })

  assert.equal(next.platforms.feishu.enabled, true)
  assert.equal(next.platforms.feishu.extra.app_id, undefined)
  assert.equal(next.platforms.feishu.extra.app_secret, undefined)
  assert.equal(next.platforms.feishu.extra.domain, 'feishu')
  assert.equal(next.platforms.feishu.extra.connection_mode, 'websocket')
  assert.equal(next.platforms.feishu.extra.webhook_path, '/feishu/webhook')
  assert.equal(next.platforms.feishu.extra.reaction_notifications, 'off')
  assert.equal(next.platforms.feishu.extra.typing_indicator, true)
  assert.equal(next.platforms.feishu.extra.resolve_sender_names, true)
})

test('Hermes 渠道保存会生成运行时仍会读取的环境变量', () => {
  const telegramEnv = buildHermesChannelEnvUpdates('telegram', {
    botToken: '123:token',
    allowFrom: '1001, 1002',
    groupAllowFrom: 'group-a\ngroup-b',
    requireMention: true,
  })

  assert.equal(telegramEnv.TELEGRAM_BOT_TOKEN, '123:token')
  assert.equal(telegramEnv.TELEGRAM_ALLOWED_USERS, '1001,1002')
  assert.equal(telegramEnv.TELEGRAM_GROUP_ALLOWED_USERS, 'group-a,group-b')
  assert.equal(telegramEnv.TELEGRAM_REQUIRE_MENTION, 'true')

  const feishuEnv = buildHermesChannelEnvUpdates('feishu', {
    appId: 'cli_xxx',
    appSecret: 'secret',
    domain: '',
    connectionMode: '',
    webhookPath: '',
    groupPolicy: 'allowlist',
    reactionNotifications: 'off',
  })

  assert.equal(feishuEnv.FEISHU_APP_ID, 'cli_xxx')
  assert.equal(feishuEnv.FEISHU_APP_SECRET, 'secret')
  assert.equal(feishuEnv.FEISHU_DOMAIN, 'feishu')
  assert.equal(feishuEnv.FEISHU_CONNECTION_MODE, 'websocket')
  assert.equal(feishuEnv.FEISHU_WEBHOOK_PATH, '/feishu/webhook')
  assert.equal(feishuEnv.FEISHU_GROUP_POLICY, 'allowlist')
  assert.equal(feishuEnv.FEISHU_REACTIONS, 'false')

  const dingTalkEnv = buildHermesChannelEnvUpdates('dingtalk', {
    clientId: 'ding-app-key',
    clientSecret: 'ding-secret',
    allowFrom: 'staff-1, staff-2',
    groupAllowFrom: 'cid-1\ncid-2',
    requireMention: true,
  })

  assert.equal(dingTalkEnv.DINGTALK_CLIENT_ID, 'ding-app-key')
  assert.equal(dingTalkEnv.DINGTALK_CLIENT_SECRET, 'ding-secret')
  assert.equal(dingTalkEnv.DINGTALK_ALLOWED_USERS, 'staff-1,staff-2')
  assert.equal(dingTalkEnv.DINGTALK_ALLOWED_CHATS, 'cid-1,cid-2')
  assert.equal(dingTalkEnv.DINGTALK_REQUIRE_MENTION, 'true')
})

test('Hermes Discord 读取会回显新版插件运行字段并优先使用环境变量', () => {
  const values = buildHermesChannelConfigValues({
    platforms: {
      discord: {
        enabled: true,
        extra: {
          require_mention: true,
          thread_require_mention: true,
          free_response_channels: ['free-a', 'free-b'],
          allowed_channels: ['allow-a'],
          ignored_channels: ['ignore-a'],
          no_thread_channels: ['plain-a'],
          auto_thread: true,
          reactions: false,
          history_backfill: true,
          history_backfill_limit: '12',
          reply_to_mode: 'all',
        },
      },
    },
  }, {
    DISCORD_BOT_TOKEN: 'env-discord-token',
    DISCORD_HOME_CHANNEL: 'home-1',
    DISCORD_HOME_CHANNEL_NAME: 'ops-home',
    DISCORD_FREE_RESPONSE_CHANNELS: 'env-free',
    DISCORD_AUTO_THREAD: 'false',
  })

  assert.equal(values.discord.enabled, true)
  assert.equal(values.discord.token, 'env-discord-token')
  assert.equal(values.discord.freeResponseChannels, 'env-free')
  assert.equal(values.discord.allowedChannels, 'allow-a')
  assert.equal(values.discord.ignoredChannels, 'ignore-a')
  assert.equal(values.discord.noThreadChannels, 'plain-a')
  assert.equal(values.discord.autoThread, false)
  assert.equal(values.discord.reactions, false)
  assert.equal(values.discord.threadRequireMention, true)
  assert.equal(values.discord.historyBackfill, true)
  assert.equal(values.discord.historyBackfillLimit, '12')
  assert.equal(values.discord.replyToMode, 'all')
  assert.equal(values.discord.homeChannel, 'home-1')
  assert.equal(values.discord.homeChannelName, 'ops-home')
})

test('Hermes Discord 保存会写入新版插件 YAML 字段和运行时环境变量', () => {
  const next = mergeHermesChannelConfig({
    platforms: {
      discord: {
        enabled: true,
        token: 'old-token',
        extra: {
          unknown_option: 'keep-me',
        },
      },
    },
  }, 'discord', {
    enabled: true,
    token: 'discord-token',
    allowFrom: '1001, 1002',
    requireMention: true,
    freeResponseChannels: 'free-a\nfree-b',
    allowedChannels: 'allow-a',
    ignoredChannels: 'ignore-a',
    noThreadChannels: 'plain-a',
    autoThread: false,
    reactions: true,
    threadRequireMention: true,
    historyBackfill: true,
    historyBackfillLimit: '12',
    replyToMode: 'off',
    homeChannel: 'home-1',
    homeChannelName: 'ops-home',
  })

  assert.equal(next.platforms.discord.enabled, true)
  assert.equal(next.platforms.discord.token, undefined)
  assert.deepEqual(next.platforms.discord.extra.allow_from, ['1001', '1002'])
  assert.deepEqual(next.platforms.discord.extra.free_response_channels, ['free-a', 'free-b'])
  assert.deepEqual(next.platforms.discord.extra.allowed_channels, ['allow-a'])
  assert.deepEqual(next.platforms.discord.extra.ignored_channels, ['ignore-a'])
  assert.deepEqual(next.platforms.discord.extra.no_thread_channels, ['plain-a'])
  assert.equal(next.platforms.discord.extra.auto_thread, false)
  assert.equal(next.platforms.discord.extra.reactions, true)
  assert.equal(next.platforms.discord.extra.thread_require_mention, true)
  assert.equal(next.platforms.discord.extra.history_backfill, true)
  assert.equal(next.platforms.discord.extra.history_backfill_limit, '12')
  assert.equal(next.platforms.discord.extra.reply_to_mode, 'off')
  assert.equal(next.platforms.discord.extra.unknown_option, 'keep-me')

  const env = buildHermesChannelEnvUpdates('discord', {
    token: 'discord-token',
    allowFrom: '1001, 1002',
    requireMention: true,
    freeResponseChannels: 'free-a\nfree-b',
    allowedChannels: 'allow-a',
    ignoredChannels: 'ignore-a',
    noThreadChannels: 'plain-a',
    autoThread: false,
    reactions: true,
    threadRequireMention: true,
    historyBackfill: true,
    historyBackfillLimit: '12',
    replyToMode: 'off',
    homeChannel: 'home-1',
    homeChannelName: 'ops-home',
  })

  assert.equal(env.DISCORD_BOT_TOKEN, 'discord-token')
  assert.equal(env.DISCORD_ALLOWED_USERS, '1001,1002')
  assert.equal(env.DISCORD_FREE_RESPONSE_CHANNELS, 'free-a,free-b')
  assert.equal(env.DISCORD_ALLOWED_CHANNELS, 'allow-a')
  assert.equal(env.DISCORD_IGNORED_CHANNELS, 'ignore-a')
  assert.equal(env.DISCORD_NO_THREAD_CHANNELS, 'plain-a')
  assert.equal(env.DISCORD_AUTO_THREAD, 'false')
  assert.equal(env.DISCORD_REACTIONS, 'true')
  assert.equal(env.DISCORD_THREAD_REQUIRE_MENTION, 'true')
  assert.equal(env.DISCORD_HISTORY_BACKFILL, 'true')
  assert.equal(env.DISCORD_HISTORY_BACKFILL_LIMIT, '12')
  assert.equal(env.DISCORD_REPLY_TO_MODE, 'off')
  assert.equal(env.DISCORD_HOME_CHANNEL, 'home-1')
  assert.equal(env.DISCORD_HOME_CHANNEL_NAME, 'ops-home')
})

test('Hermes 渠道保存会从 YAML 清理旧凭证，避免覆盖 .env 运行时值', () => {
  const next = mergeHermesChannelConfig({
    platforms: {
      slack: {
        enabled: true,
        token: 'old-bot-token',
        extra: {
          app_token: 'old-app-token',
          signing_secret: 'old-signing-secret',
          webhook_path: '/old/events',
          unknown_option: 'keep-me',
        },
      },
    },
  }, 'slack', {
    enabled: true,
    botToken: 'xoxb-new',
    appToken: 'xapp-new',
    signingSecret: 'new-signing-secret',
    webhookPath: '/slack/events',
  })

  assert.equal(next.platforms.slack.token, undefined)
  assert.equal(next.platforms.slack.extra.app_token, undefined)
  assert.equal(next.platforms.slack.extra.signing_secret, undefined)
  assert.equal(next.platforms.slack.extra.webhook_path, '/slack/events')
  assert.equal(next.platforms.slack.extra.unknown_option, 'keep-me')
})

test('Hermes 钉钉保存会使用运行时实际读取的字段', () => {
  const next = mergeHermesChannelConfig({
    platforms: {
      dingtalk: {
        enabled: true,
        extra: {
          client_id: 'old-client-id',
          client_secret: 'old-client-secret',
          group_allow_from: ['legacy-chat'],
          unknown_option: 'keep-me',
        },
      },
    },
  }, 'dingtalk', {
    enabled: true,
    clientId: 'ding-app-key',
    clientSecret: 'ding-secret',
    allowFrom: 'staff-1, staff-2',
    groupAllowFrom: 'cid-1\ncid-2',
    requireMention: true,
  })

  assert.equal(next.platforms.dingtalk.enabled, true)
  assert.equal(next.platforms.dingtalk.extra.client_id, undefined)
  assert.equal(next.platforms.dingtalk.extra.client_secret, undefined)
  assert.equal(next.platforms.dingtalk.extra.group_allow_from, undefined)
  assert.deepEqual(next.platforms.dingtalk.extra.allowed_users, ['staff-1', 'staff-2'])
  assert.deepEqual(next.platforms.dingtalk.extra.allowed_chats, ['cid-1', 'cid-2'])
  assert.equal(next.platforms.dingtalk.extra.require_mention, true)
  assert.equal(next.platforms.dingtalk.extra.unknown_option, 'keep-me')
})
