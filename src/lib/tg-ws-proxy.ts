export const DEFAULT_TG_WS_PROXY_HOST = '127.0.0.1'
export const DEFAULT_TG_WS_PROXY_PORT = 1443

export function normalizeTgWsProxySecret(secret?: string) {
  return secret?.trim().toLowerCase() ?? ''
}

export function isValidTgWsProxySecret(secret?: string) {
  const normalized = normalizeTgWsProxySecret(secret)
  return normalized.length === 32 && /^[0-9a-f]+$/i.test(normalized)
}

export function generateTgWsProxySecret() {
  return crypto.randomUUID().replace(/-/g, '')
}

export function buildTgWsProxyLink(port: number, secret: string, host = DEFAULT_TG_WS_PROXY_HOST) {
  const params = new URLSearchParams({
    server: host,
    port: String(port),
    secret: `dd${normalizeTgWsProxySecret(secret)}`,
  })
  return `tg://proxy?${params.toString()}`
}

export function buildTgWsProxyHttpLink(port: number, secret: string, host = DEFAULT_TG_WS_PROXY_HOST) {
  const params = new URLSearchParams({
    server: host,
    port: String(port),
    secret: `dd${normalizeTgWsProxySecret(secret)}`,
  })
  return `https://t.me/proxy?${params.toString()}`
}
