import type ru from '@/locales/ru.json'

declare module 'i18next' {
  interface CustomTypeOptions {
    defaultNS: 'translation'
    resources: {
      translation: typeof ru
    }
  }
}
