// @ts-nocheck
export const ZHIZHUA_SERVICE_URL = (typeof __ZHIZHUA_SERVICE_URL__ === 'string' ? __ZHIZHUA_SERVICE_URL__ : 'https://ai.iazp.cn').replace(/\/+$/, '')
export const ZHIZHUA_MODEL_BASE_URL = (typeof __ZHIZHUA_MODEL_BASE_URL__ === 'string' ? __ZHIZHUA_MODEL_BASE_URL__ : `${ZHIZHUA_SERVICE_URL}/v1`).replace(/\/+$/, '')
export const ZHIZHUA_SERVICE_HOST = ZHIZHUA_SERVICE_URL.replace(/^https?:\/\//, '')

export function zhizhuaUrl(path = '') {
  if (!path) return ZHIZHUA_SERVICE_URL
  return `${ZHIZHUA_SERVICE_URL}${path.startsWith('/') ? path : `/${path}`}`
}
