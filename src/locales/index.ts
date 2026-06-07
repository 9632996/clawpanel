// @ts-nocheck
/**
 * 语言包聚合入口
 * 从 modules/ 导入所有模块，按语言合并输出
 */
import { SUPPORTED_LANGS } from './helper.ts'
import common from './modules/common.ts'
import sidebar from './modules/sidebar.ts'
import instance from './modules/instance.ts'
import dashboard from './modules/dashboard.ts'
import services from './modules/services.ts'
import settings from './modules/settings.ts'
import models from './modules/models.ts'
import agents from './modules/agents.ts'
import agentDetail from './modules/agentDetail.ts'
import gateway from './modules/gateway.ts'
import security from './modules/security.ts'
import communication from './modules/communication.ts'
import channels from './modules/channels.ts'
import memory from './modules/memory.ts'
import dreaming from './modules/dreaming.ts'
import cron from './modules/cron.ts'
import usage from './modules/usage.ts'
import skills from './modules/skills.ts'
import chat from './modules/chat.ts'
import chatDebug from './modules/chat-debug.ts'
import setup from './modules/setup.ts'
import about from './modules/about.ts'
import ext from './modules/ext.ts'
import logs from './modules/logs.ts'
import assistant from './modules/assistant.ts'
import toast from './modules/toast.ts'
import modal from './modules/modal.ts'
import engagement from './modules/engagement.ts'
import diagnose from './modules/diagnose.ts'
import routeMap from './modules/routeMap.ts'
import extensions from './modules/extensions.ts'
import engine from './modules/engine.ts'
import ciaoBug from './modules/ciaoBug.ts'
import cliConflict from './modules/cliConflict.ts'
import glossary from './modules/glossary.ts'
import hermesLazyDeps from './modules/hermesLazyDeps.ts'
import notifications from './modules/notifications.ts'
import kernel from './modules/kernel.ts'

const MODULES = {
  common, sidebar, instance, dashboard, services, settings,
  models, agents, agentDetail, gateway, security, communication, channels,
  memory, dreaming, cron, usage, skills, chat, chatDebug, setup, about,
  ext, logs, assistant, toast, modal, engagement, diagnose, routeMap, extensions,
  engine, ciaoBug, cliConflict, glossary, hermesLazyDeps, notifications, kernel,
}

/** 判断是否是 _() 调用产生的翻译对象（有 'zh-CN' 字符串字段） */
function _isTranslationObject(v) {
  return v && typeof v === 'object' && typeof v['zh-CN'] === 'string'
}

/** 递归 materialize：把翻译对象转成当前语言的字符串，嵌套对象继续递归 */
function _materialize(entries, lang) {
  const out = {}
  for (const [key, val] of Object.entries(entries)) {
    if (_isTranslationObject(val)) {
      out[key] = val[lang] || val['zh-CN'] || key
    } else if (val && typeof val === 'object' && !Array.isArray(val)) {
      // 嵌套字典（如 common.errorHint.{generic,network,...}）— 递归
      out[key] = _materialize(val, lang)
    } else {
      out[key] = val
    }
  }
  return out
}

/** 构建所有语言字典 { 'zh-CN': { common: {...}, sidebar: {...}, ... }, ... } */
export function buildLocales() {
  const result = {}
  for (const lang of SUPPORTED_LANGS) {
    result[lang] = {}
    for (const [mod, entries] of Object.entries(MODULES)) {
      result[lang][mod] = _materialize(entries, lang)
    }
  }
  return result
}
