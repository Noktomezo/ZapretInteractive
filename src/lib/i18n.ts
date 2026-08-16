import i18n from 'i18next'
import { initReactI18next } from 'react-i18next'
import ru from '@/locales/ru.json'

export const defaultNS = 'translation'
export const resources = {
  ru: {
    translation: ru,
  },
} as const

void i18n
  .use(initReactI18next)
  .init({
    lng: 'ru',
    fallbackLng: 'ru',
    defaultNS,
    resources,
    interpolation: {
      escapeValue: false,
    },
  })

// fallow-ignore-next-line unused-export
export default i18n
