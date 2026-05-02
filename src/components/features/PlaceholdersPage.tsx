import type { Placeholder } from '@/lib/types'
import { FileCode, FilePenLine, FolderOpen, Loader2, Package, Pencil, Plus, RefreshCcw, RotateCcw, Trash2, UserRoundPlus } from 'lucide-react'
import { useRef, useState } from 'react'
import { toast } from 'sonner'
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { InlineMarker } from '@/components/ui/inline-marker'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { LenisScrollArea } from '@/components/ui/lenis-scroll-area'
import { useMountEffect } from '@/hooks/use-mount-effect'
import { buildRestoredPlaceholder, getBuiltinPlaceholder, isSystemPlaceholder, isSystemPlaceholderModified, isSystemPlaceholderUpdateAvailable } from '@/lib/system-config'
import * as tauri from '@/lib/tauri'
import { useConfigStore } from '@/stores/config.store'
import { useConnectionStore } from '@/stores/connection.store'

const RESOURCES_ALIAS_PREFIX = '@resources'
const LEADING_RESOURCE_SEPARATORS = /^[/\\]+/
const PATH_SEGMENT_SEPARATOR = /[/\\]+/g
const TRAILING_SLASHES_RE = /[/\\]+$/

function isResourcesAliasPath(path: string) {
  const lowerCasePath = path.toLowerCase()
  if (!lowerCasePath.startsWith(RESOURCES_ALIAS_PREFIX)) {
    return false
  }

  const nextCharacter = path[RESOURCES_ALIAS_PREFIX.length]
  return nextCharacter === undefined || nextCharacter === '/' || nextCharacter === '\\'
}

export function PlaceholdersPage() {
  const [editingIndex, setEditingIndex] = useState<number | null>(null)
  const [editName, setEditName] = useState('')
  const [editPath, setEditPath] = useState('')
  const [addOpen, setAddOpen] = useState(false)
  const [newName, setNewName] = useState('')
  const [newPath, setNewPath] = useState('')
  const [resourcesDir, setResourcesDir] = useState('')
  const [systemPlaceholderTarget, setSystemPlaceholderTarget] = useState<Placeholder | null>(null)
  const isSavingRef = useRef(false)

  const config = useConfigStore(state => state.config)
  const builtinConfig = useConfigStore(state => state.builtinConfig)
  const loading = useConfigStore(state => state.loading)
  const load = useConfigStore(state => state.load)
  const saveNow = useConfigStore(state => state.saveNow)
  const addPlaceholder = useConfigStore(state => state.addPlaceholder)
  const revertTo = useConfigStore(state => state.revertTo)
  const updatePlaceholder = useConfigStore(state => state.updatePlaceholder)
  const replacePlaceholdersState = useConfigStore(state => state.replacePlaceholdersState)
  const addConfigLog = useConnectionStore(state => state.addConfigLog)

  useMountEffect(() => {
    void load().catch(console.error)
    void tauri.getResourcesDirectory()
      .then(setResourcesDir)
      .catch((error) => {
        console.error('Failed to get resources directory:', error)
      })
  })

  const resolvePlaceholderPath = (path: string) => {
    const trimmedPath = path.trim()
    if (!trimmedPath) {
      return ''
    }

    if (!isResourcesAliasPath(trimmedPath)) {
      return trimmedPath
    }

    const relativePath = trimmedPath
      .slice(RESOURCES_ALIAS_PREFIX.length)
      .replace(LEADING_RESOURCE_SEPARATORS, '')
      .replace(PATH_SEGMENT_SEPARATOR, '\\')

    if (!resourcesDir) {
      return relativePath ? `${RESOURCES_ALIAS_PREFIX}\\${relativePath}` : RESOURCES_ALIAS_PREFIX
    }

    return relativePath ? `${resourcesDir}\\${relativePath}` : resourcesDir
  }

  const toStoredPlaceholderPath = (path: string) => {
    const trimmedPath = path.trim()
    if (!trimmedPath) {
      return trimmedPath
    }

    if (isResourcesAliasPath(trimmedPath)) {
      const relativePath = trimmedPath
        .slice(RESOURCES_ALIAS_PREFIX.length)
        .replace(LEADING_RESOURCE_SEPARATORS, '')
        .replace(PATH_SEGMENT_SEPARATOR, '/')

      return relativePath ? `${RESOURCES_ALIAS_PREFIX}/${relativePath}` : RESOURCES_ALIAS_PREFIX
    }

    if (!resourcesDir) {
      return trimmedPath
    }

    const normalizedResourcesDir = resourcesDir
      .replace(PATH_SEGMENT_SEPARATOR, '/')
      .replace(TRAILING_SLASHES_RE, '')
      .toLowerCase()
    const normalizedPath = trimmedPath.replace(PATH_SEGMENT_SEPARATOR, '/')

    if (normalizedPath.toLowerCase() === normalizedResourcesDir) {
      return RESOURCES_ALIAS_PREFIX
    }

    const resourcesPrefix = `${normalizedResourcesDir}/`
    if (!normalizedPath.toLowerCase().startsWith(resourcesPrefix)) {
      return trimmedPath
    }

    const relativePath = normalizedPath.slice(resourcesPrefix.length)
    return relativePath ? `${RESOURCES_ALIAS_PREFIX}/${relativePath}` : RESOURCES_ALIAS_PREFIX
  }

  const validatePlaceholder = (name: string, path: string, excludedIndex?: number) => {
    const normalizedName = name.trim().toLocaleLowerCase()
    const normalizedPath = toStoredPlaceholderPath(path).trim().toLocaleLowerCase()
    const placeholders = useConfigStore.getState().config?.placeholders ?? []

    if (placeholders.some((placeholder, index) => index !== excludedIndex && placeholder.name.trim().toLocaleLowerCase() === normalizedName)) {
      toast.error('Плейсхолдер с таким названием уже существует')
      return false
    }

    if (placeholders.some((placeholder, index) => index !== excludedIndex && placeholder.path.trim().toLocaleLowerCase() === normalizedPath)) {
      toast.error('Плейсхолдер с таким путём уже существует')
      return false
    }

    return true
  }

  const handleAdd = async () => {
    if (isSavingRef.current) {
      toast.error('Подождите, выполняется сохранение')
      return
    }

    if (!newName.trim() || !newPath.trim()) {
      return
    }

    const currentConfig = useConfigStore.getState().config
    if (!currentConfig) {
      return
    }

    const previousConfig = structuredClone(currentConfig)
    const placeholderName = newName.trim()
    const placeholderPath = toStoredPlaceholderPath(newPath.trim())
    if (!validatePlaceholder(placeholderName, placeholderPath)) {
      return
    }
    addPlaceholder(placeholderName, placeholderPath)
    isSavingRef.current = true
    try {
      await saveNow()
      addConfigLog(`добавлен плейсхолдер "{{${placeholderName}}}" -> ${placeholderPath}`)
      setNewName('')
      setNewPath('')
      setAddOpen(false)
      toast.success('Плейсхолдер добавлен')
    }
    catch (e) {
      revertTo(previousConfig)
      toast.error(`Ошибка сохранения плейсхолдера: ${e instanceof Error ? e.message : String(e)}`)
    }
    finally {
      isSavingRef.current = false
    }
  }

  const handleEdit = (index: number, placeholder: Placeholder) => {
    setEditingIndex(index)
    setEditName(placeholder.name)
    setEditPath(resolvePlaceholderPath(placeholder.path))
  }

  const handleOpenAppDirectory = async () => {
    try {
      await tauri.openAppDirectory()
    }
    catch (error) {
      console.error('Failed to open app directory:', error)
      toast.error(`Не удалось открыть папку приложения: ${error instanceof Error ? error.message : String(error)}`)
    }
  }

  const handleSaveEdit = async () => {
    if (isSavingRef.current) {
      toast.error('Подождите, выполняется сохранение')
      return
    }

    if (editingIndex === null) {
      return
    }

    const trimmedName = editName.trim()
    const trimmedPath = toStoredPlaceholderPath(editPath)
    if (!trimmedName || !trimmedPath) {
      return
    }

    const currentConfig = useConfigStore.getState().config
    if (!currentConfig) {
      return
    }

    const previousConfig = structuredClone(currentConfig)
    const previousPlaceholder = previousConfig.placeholders[editingIndex]
    if (!validatePlaceholder(trimmedName, trimmedPath, editingIndex)) {
      return
    }
    updatePlaceholder(editingIndex, trimmedName, trimmedPath)
    isSavingRef.current = true
    try {
      await saveNow()
      if (previousPlaceholder) {
        addConfigLog(
          previousPlaceholder.name !== trimmedName
            ? `обновлён плейсхолдер "{{${previousPlaceholder.name}}}" -> "{{${trimmedName}}}"`
            : `путь плейсхолдера "{{${trimmedName}}}" изменён на ${trimmedPath}`,
        )
      }
      setEditingIndex(null)
      toast.success('Плейсхолдер сохранён')
    }
    catch (e) {
      revertTo(previousConfig)
      toast.error(`Ошибка сохранения плейсхолдера: ${e instanceof Error ? e.message : String(e)}`)
    }
    finally {
      isSavingRef.current = false
    }
  }

  const handleDelete = async (index: number) => {
    if (isSavingRef.current)
      return
    const prevPlaceholders = config?.placeholders?.slice() ?? []
    const prevRemovedNames = config?.systemRemovedPlaceholderNames ?? []
    const deletedPlaceholder = prevPlaceholders[index]
    const nextPlaceholders = prevPlaceholders.filter((_, currentIndex) => currentIndex !== index)
    const nextRemovedNames = deletedPlaceholder?.system
      ? Array.from(new Set([...(config?.systemRemovedPlaceholderNames ?? []), deletedPlaceholder.systemBaseName ?? deletedPlaceholder.name]))
      : (config?.systemRemovedPlaceholderNames ?? [])
    replacePlaceholdersState(nextPlaceholders, nextRemovedNames)
    isSavingRef.current = true
    try {
      await saveNow()
      if (deletedPlaceholder) {
        addConfigLog(`удалён плейсхолдер "{{${deletedPlaceholder.name}}}"`)
      }
      toast.success('Плейсхолдер удалён')
    }
    catch (e) {
      toast.error(`Ошибка сохранения: ${e}`)
      replacePlaceholdersState(prevPlaceholders, prevRemovedNames)
    }
    finally {
      isSavingRef.current = false
    }
  }

  const handleRestorePlaceholder = async () => {
    if (!systemPlaceholderTarget || !config) {
      return
    }

    const builtinPlaceholder = getBuiltinPlaceholder(
      builtinConfig,
      systemPlaceholderTarget.name,
      systemPlaceholderTarget.systemBaseName,
    )
    if (!builtinPlaceholder) {
      return
    }

    const placeholderIndex = config.placeholders.findIndex(placeholder => placeholder.name === systemPlaceholderTarget.name)
    if (placeholderIndex < 0) {
      return
    }

    const previousPlaceholders = structuredClone(config.placeholders)
    const nextPlaceholders = structuredClone(config.placeholders)
    nextPlaceholders[placeholderIndex] = buildRestoredPlaceholder(builtinPlaceholder)
    const nextRemovedNames = (config.systemRemovedPlaceholderNames ?? []).filter(name => name !== builtinPlaceholder.name)
    replacePlaceholdersState(nextPlaceholders, nextRemovedNames)
    isSavingRef.current = true
    try {
      await saveNow()
      addConfigLog(`плейсхолдер "{{${systemPlaceholderTarget.name}}}" обновлён до системного значения`)
      toast.success('Плейсхолдер обновлён')
    }
    catch (error) {
      replacePlaceholdersState(previousPlaceholders, config.systemRemovedPlaceholderNames ?? [])
      toast.error(`Ошибка обновления плейсхолдера: ${error instanceof Error ? error.message : String(error)}`)
    }
    finally {
      setSystemPlaceholderTarget(null)
      isSavingRef.current = false
    }
  }

  if (loading) {
    return (
      <div className="flex h-full items-center justify-center">
        <Loader2 className="h-6 w-6 animate-spin" />
      </div>
    )
  }

  return (
    <LenisScrollArea className="h-full min-h-0 min-w-0">
      <div className="w-full min-w-0 max-w-full space-y-6 overflow-x-hidden p-6">
        <div className="flex min-w-0 items-center justify-between">
          <div className="min-w-0 flex-1">
            <h1 className="text-2xl font-medium">Плейсхолдеры</h1>
            <p className="mt-1 text-sm text-muted-foreground">
              Замена плейсхолдеров на пути к бинарным файлам
            </p>
          </div>
          <div className="ml-4 flex shrink-0 items-center gap-1">
            <Button onClick={() => setAddOpen(true)}>
              <Plus className="h-4 w-4" />
              Новый плейсхолдер
            </Button>
            <Button
              variant="outline"
              size="icon"
              onClick={() => void handleOpenAppDirectory()}
              title="Открыть папку приложения"
              aria-label="Открыть папку приложения"
            >
              <FolderOpen className="h-4 w-4" />
            </Button>
          </div>
        </div>

        <div className="min-w-0 space-y-3">
          {!config || config.placeholders.length === 0
            ? (
                <div className="text-muted-foreground flex min-h-32 items-center justify-center rounded-lg border border-dashed">
                  Нет плейсхолдеров
                </div>
              )
            : (
                config.placeholders.map((placeholder: Placeholder, index: number) => {
                  const builtin = getBuiltinPlaceholder(builtinConfig, placeholder.name, placeholder.systemBaseName)
                  const isSystem = isSystemPlaceholder(placeholder)
                  const isModified = isSystemPlaceholderModified(placeholder)
                  const hasUpdate = isSystemPlaceholderUpdateAvailable(placeholder, builtin)

                  return (
                    <div
                      key={`${index}-${placeholder.name}`}
                      className="bg-card flex min-h-[4.5rem] items-center justify-between gap-4 overflow-hidden rounded-lg border px-4 py-3"
                    >
                      <div className="flex min-w-0 w-0 flex-1 items-center gap-3 overflow-hidden">
                        <div className="text-muted-foreground flex size-9 shrink-0 items-center justify-center rounded-md border border-border/70 bg-muted/25">
                          <FileCode className="h-4 w-4" />
                        </div>
                        <div className="min-w-0 w-0 flex-1 overflow-hidden space-y-1">
                          <div className="flex items-center gap-1 truncate text-sm font-normal text-foreground">
                            {'{{'}
                            {placeholder.name}
                            {'}}'}
                            <div className="flex items-center gap-1 text-muted-foreground">
                              {isSystem
                                ? <InlineMarker icon={Package} label="Системный плейсхолдер" />
                                : <InlineMarker icon={UserRoundPlus} label="Пользовательский плейсхолдер" className="text-primary/80" />}
                              {isModified && (
                                <InlineMarker icon={FilePenLine} label="Системный плейсхолдер изменён пользователем" className="text-warning" />
                              )}
                              {isSystem && (isModified || hasUpdate) && (
                                <InlineMarker
                                  icon={hasUpdate ? RefreshCcw : RotateCcw}
                                  label={hasUpdate
                                    ? 'Обновить плейсхолдер до актуального системного значения'
                                    : 'Откатить плейсхолдер к системному значению'}
                                  className={hasUpdate ? 'text-primary' : 'text-destructive'}
                                  onClick={() => setSystemPlaceholderTarget(placeholder)}
                                />
                              )}
                            </div>
                          </div>
                          <div className="truncate overflow-hidden text-xs text-muted-foreground/90" title={resolvePlaceholderPath(placeholder.path)}>
                            {resolvePlaceholderPath(placeholder.path)}
                          </div>
                        </div>
                      </div>
                      <div className="flex shrink-0 items-center gap-1">
                        <Button
                          variant="outline"
                          size="icon"
                          aria-label={`Редактировать плейсхолдер ${placeholder.name}`}
                          onClick={() => handleEdit(index, placeholder)}
                        >
                          <Pencil className="h-4 w-4" />
                        </Button>
                        <Button
                          variant="outline"
                          size="icon"
                          className="bg-destructive/10 text-destructive hover:bg-destructive/18"
                          aria-label={`Удалить плейсхолдер ${placeholder.name}`}
                          onClick={() => handleDelete(index)}
                        >
                          <Trash2 className="h-4 w-4" />
                        </Button>
                      </div>
                    </div>
                  )
                })
              )}
        </div>

        <Dialog open={addOpen} onOpenChange={setAddOpen}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Новый плейсхолдер</DialogTitle>
            </DialogHeader>
            <div className="space-y-4 py-4">
              <Input
                aria-label="Название плейсхолдера"
                placeholder="Название"
                value={newName}
                onChange={e => setNewName(e.target.value)}
              />
              <Input
                aria-label="Путь плейсхолдера"
                placeholder="Путь к файлу"
                value={newPath}
                onChange={e => setNewPath(e.target.value)}
              />
              {newPath.trim() && (
                <p className="text-xs text-muted-foreground break-all">
                  {resolvePlaceholderPath(toStoredPlaceholderPath(newPath))}
                </p>
              )}
            </div>
            <DialogFooter>
              <Button variant="outline" onClick={() => setAddOpen(false)}>
                Отмена
              </Button>
              <Button onClick={handleAdd}>Добавить</Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>

        <Dialog open={editingIndex !== null} onOpenChange={open => !open && setEditingIndex(null)}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Редактировать плейсхолдер</DialogTitle>
            </DialogHeader>
            <div className="space-y-4 py-4">
              <div className="space-y-2">
                <Label htmlFor="edit-placeholder-name">Название</Label>
                <Input
                  id="edit-placeholder-name"
                  aria-label="Название плейсхолдера"
                  placeholder="Название"
                  value={editName}
                  onChange={e => setEditName(e.target.value)}
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="edit-placeholder-path">Путь</Label>
                <Input
                  id="edit-placeholder-path"
                  aria-label="Путь плейсхолдера"
                  placeholder="Путь к файлу"
                  value={editPath}
                  onChange={e => setEditPath(e.target.value)}
                />
              </div>
            </div>
            <DialogFooter>
              <Button variant="outline" onClick={() => setEditingIndex(null)}>
                Отмена
              </Button>
              <Button onClick={handleSaveEdit}>Сохранить</Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>

        <AlertDialog open={!!systemPlaceholderTarget} onOpenChange={open => !open && setSystemPlaceholderTarget(null)}>
          <AlertDialogContent>
            <AlertDialogHeader>
              <AlertDialogTitle>
                {systemPlaceholderTarget && isSystemPlaceholderUpdateAvailable(
                  systemPlaceholderTarget,
                  getBuiltinPlaceholder(builtinConfig, systemPlaceholderTarget.name, systemPlaceholderTarget.systemBaseName),
                )
                  ? 'Обновить системный плейсхолдер?'
                  : 'Откатить плейсхолдер к системному значению?'}
              </AlertDialogTitle>
              <AlertDialogDescription>
                {systemPlaceholderTarget
                  ? `Плейсхолдер "{{${systemPlaceholderTarget.name}}}" будет возвращён к актуальному системному значению.`
                  : ''}
              </AlertDialogDescription>
            </AlertDialogHeader>
            <AlertDialogFooter>
              <AlertDialogCancel>Отмена</AlertDialogCancel>
              <AlertDialogAction onClick={() => void handleRestorePlaceholder()}>
                Обновить
              </AlertDialogAction>
            </AlertDialogFooter>
          </AlertDialogContent>
        </AlertDialog>
      </div>
    </LenisScrollArea>
  )
}
