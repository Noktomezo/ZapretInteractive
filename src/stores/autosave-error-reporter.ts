import { toast } from 'sonner'

export type AutosaveErrorLogger = (message: string) => void

let autosaveErrorLogger: AutosaveErrorLogger | null = null
let lastAutosaveErrorKey: string | null = null

function getErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error)
}

export function setAutosaveErrorLogger(logger: AutosaveErrorLogger | null) {
  autosaveErrorLogger = logger
}

export function resetAutosaveErrorReporter() {
  autosaveErrorLogger = null
  lastAutosaveErrorKey = null
}

export function reportAutosaveError(reason: string, error: unknown) {
  const message = getErrorMessage(error)
  const dedupeKey = `${reason}:${message}`
  if (lastAutosaveErrorKey === dedupeKey) {
    return
  }

  lastAutosaveErrorKey = dedupeKey
  autosaveErrorLogger?.(`Автосохранение настроек завершилось ошибкой (${reason}): ${message}`)
  toast.error('Изменения не удалось сохранить автоматически')
}
