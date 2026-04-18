export const DEFAULT_DNS_PRESET_ID = 'comss-one'
export const DEFAULT_BOOTSTRAP_RESOLVER = '77.88.8.8'

export const DNS_PRESETS = [
  {
    id: 'comss-one',
    name: 'Comss',
    urls: ['https://dns.comss.one/dns-query'],
  },
  {
    id: 'xbox-dns-ru',
    name: 'Xbox DNS',
    urls: ['https://xbox-dns.ru/dns-query'],
  },
  {
    id: 'malw-link-main',
    name: 'Malw Link',
    urls: ['https://dns.malw.link/dns-query'],
  },
  {
    id: 'malw-link-cf',
    name: 'Malw Link (Cloudflare)',
    urls: ['https://5u35p8m9i7.cloudflare-gateway.com/dns-query'],
  },
  {
    id: 'mafioznik',
    name: 'Mafioznik',
    urls: ['https://dns.mafioznik.xyz/dns-query'],
  },
  {
    id: 'astracat',
    name: 'Astracat',
    urls: ['https://dns.astracat.ru/dns-query'],
  },
] as const

export const BOOTSTRAP_RESOLVER_OPTIONS = [
  { value: '77.88.8.8', label: 'Yandex · 77.88.8.8' },
  { value: '1.1.1.1', label: 'Cloudflare · 1.1.1.1' },
  { value: '8.8.8.8', label: 'Google · 8.8.8.8' },
] as const

export function normalizeDnsPresetId(presetId?: string) {
  const normalizedPresetId = presetId?.trim()
  const normalizedPresetKey = normalizedPresetId?.toLowerCase()
  if (normalizedPresetKey === 'malw-link') {
    return 'malw-link-main'
  }

  if (!normalizedPresetKey) {
    return DEFAULT_DNS_PRESET_ID
  }

  return DNS_PRESETS.find(preset => preset.id.toLowerCase() === normalizedPresetKey)?.id ?? DEFAULT_DNS_PRESET_ID
}

export function applyDnsAccelerator(urls: string[], enabled: boolean) {
  if (!enabled) {
    return urls
  }

  return urls.map((url) => {
    try {
      const parsed = new URL(url)
      return `https://v.recipes/dns/${parsed.host}${parsed.pathname}${parsed.search}${parsed.hash}`
    }
    catch {
      return url
    }
  })
}

export function getDnsLatencyBadgeClass(latency: number | null) {
  if (latency === null) {
    return 'border-destructive/35 bg-destructive/8 text-destructive'
  }
  if (latency <= 30) {
    return 'border-success/35 bg-success/8 text-success'
  }
  if (latency <= 80) {
    return 'border-warning/35 bg-warning/10 text-warning'
  }
  return 'border-destructive/35 bg-destructive/8 text-destructive'
}
