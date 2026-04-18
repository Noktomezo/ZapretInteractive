export const DEFAULT_TG_WS_PROXY_HOST = '127.0.0.1'
export const DEFAULT_TG_WS_PROXY_PORT = 1443
const TG_WS_PROXY_SECRET_RE = /^[a-f0-9]{32}$/

export function normalizeTgWsProxySecret(secret?: string) {
  return secret?.trim().toLowerCase() ?? ''
}

export function isValidTgWsProxySecret(secret?: string) {
  return TG_WS_PROXY_SECRET_RE.test(normalizeTgWsProxySecret(secret))
}

export function generateTgWsProxySecret() {
  return crypto.randomUUID().replace(/-/g, '')
}

function buildTgWsProxyParams(port: number, secret: string, host = DEFAULT_TG_WS_PROXY_HOST) {
  return new URLSearchParams({
    server: host,
    port: String(port),
    secret: `dd${normalizeTgWsProxySecret(secret)}`,
  })
}

export function buildTgWsProxyLink(port: number, secret: string, host = DEFAULT_TG_WS_PROXY_HOST) {
  const params = buildTgWsProxyParams(port, secret, host)
  return `tg://proxy?${params.toString()}`
}

export function buildTgWsProxyHttpLink(port: number, secret: string, host = DEFAULT_TG_WS_PROXY_HOST) {
  const params = buildTgWsProxyParams(port, secret, host)
  return `https://t.me/proxy?${params.toString()}`
}
