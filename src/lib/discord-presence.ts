import type { AppConfig } from './types'

type ConnectionStatus = 'disconnected' | 'connecting' | 'connected' | 'disconnecting' | 'error'

function safeDecodePathSegment(value: string) {
  try {
    return decodeURIComponent(value)
  }
  catch {
    return value
  }
}

export function getDiscordPresenceDetails(pathname: string, config: AppConfig | null) {
  if (pathname === '/') {
    return 'Главная'
  }
  if (pathname === '/about') {
    return 'О программе'
  }
  if (pathname === '/settings') {
    return 'Настройки'
  }
  if (pathname === '/logs') {
    return 'Логи'
  }
  if (pathname === '/filters') {
    return 'Фильтры'
  }
  if (pathname === '/placeholders') {
    return 'Плейсхолдеры'
  }
  if (pathname === '/modules') {
    return 'Модули'
  }
  if (pathname === '/modules/dns') {
    return 'Модули: DNS'
  }
  if (pathname === '/modules/tg-ws-proxy') {
    return 'Модули: TG WS Proxy'
  }
  if (pathname === '/strategies') {
    return 'Категории стратегий'
  }
  if (pathname.startsWith('/strategies/')) {
    const categoryId = safeDecodePathSegment(pathname.slice('/strategies/'.length))
    const categoryName = config?.categories.find(category => category.id === categoryId)?.name
    return categoryName ? `Стратегии: ${categoryName}` : 'Стратегии'
  }

  return 'Zapret Interactive'
}

export function getDiscordPresenceState(status: ConnectionStatus) {
  switch (status) {
    case 'connected':
      return 'Подключено'
    case 'connecting':
      return 'Подключение...'
    case 'disconnecting':
      return 'Отключение...'
    case 'error':
      return 'Ошибка подключения'
    case 'disconnected':
    default:
      return 'Отключено'
  }
}
