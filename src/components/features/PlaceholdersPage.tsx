import type { Placeholder } from '@/lib/types'
import { FileCode, FilePenLine, FolderOpen, Loader2, Package, Pencil, Plus, RefreshCcw, RotateCcw, Trash2, UserRoundPlus } from 'lucide-react'
import { useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
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
import { ScrollTopButton } from '@/components/ui/scroll-top-button'
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
  const { t } = useTranslation()
  const scrollAreaRef = useRef<HTMLDivElement>(null)
  const config = useConfigStore(state => state.config)
  const builtinConfig = useConfigStore(state => state.builtinConfig)
  const loading = useConfigStore(state => state.loading)
  const load = useConfigStore(state => state.load)
  const saveNow = useConfigStore(state => state.saveNow)
  const revertTo = useConfigStore(state => state.revertTo)
  const addPlaceholder = useConfigStore(state => state.addPlaceholder)
  const updatePlaceholder = useConfigStore(state => state.updatePlaceholder)
  const replacePlaceholdersState = useConfigStore(state => state.replacePlaceholdersState)
  const addConfigLog = useConnectionStore(state => state.addConfigLog)

  const [addOpen, setAddOpen] = useState(false)
  const [newName, setNewName] = useState('')
  const [newPath, setNewPath] = useState('')
  const [resourcesDir, setResourcesDir] = useState('')

  const [editingIndex, setEditingIndex] = useState<number | null>(null)
  const [editName, setEditName] = useState('')
  const [editPath, setEditPath] = useState('')
  const [systemPlaceholderTarget, setSystemPlaceholderTarget] = useState<Placeholder | null>(null)
  const isSavingRef = useRef(false)

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
      toast.error(t('placeholders.nameExists'))
      return false
    }

    if (placeholders.some((placeholder, index) => index !== excludedIndex && placeholder.path.trim().toLocaleLowerCase() === normalizedPath)) {
      toast.error(t('placeholders.pathExists'))
      return false
    }

    return true
  }

  const handleAdd = async () => {
    if (!newName.trim() || !newPath.trim() || isSavingRef.current)
      return

    const storedPath = toStoredPlaceholderPath(newPath)
    if (!validatePlaceholder(newName, storedPath)) {
      return
    }

    const currentConfig = useConfigStore.getState().config
    if (!currentConfig) {
      return
    }

    const previousConfig = structuredClone(currentConfig)
    const nameToAdd = newName.trim()

    isSavingRef.current = true
    addPlaceholder(nameToAdd, storedPath)

    try {
      await saveNow()
      addConfigLog(`добавлен плейсхолдер "${nameToAdd}"`)
      setNewName('')
      setNewPath('')
      setAddOpen(false)
      toast.success(t('placeholders.placeholderAdded'))
    }
    catch (e) {
      revertTo(previousConfig)
      toast.error(`Ошибка сохранения: ${e instanceof Error ? e.message : String(e)}`)
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

  const handleSaveEdit = async () => {
    if (editingIndex === null || !editName.trim() || !editPath.trim() || isSavingRef.current)
      return

    const storedPath = toStoredPlaceholderPath(editPath)
    if (!validatePlaceholder(editName, storedPath, editingIndex)) {
      return
    }

    const currentConfig = useConfigStore.getState().config
    if (!currentConfig) {
      return
    }

    const previousConfig = structuredClone(currentConfig)
    const previousName = currentConfig.placeholders[editingIndex]?.name ?? editName
    const nextName = editName.trim()

    isSavingRef.current = true
    updatePlaceholder(editingIndex, nextName, storedPath)

    try {
      await saveNow()
      addConfigLog(
        previousName !== nextName
          ? `плейсхолдер "${previousName}" переименован в "${nextName}"`
          : `обновлен плейсхолдер "${previousName}"`,
      )
      setEditingIndex(null)
      toast.success(t('placeholders.placeholderSaved'))
    }
    catch (e) {
      revertTo(previousConfig)
      toast.error(`Ошибка сохранения: ${e instanceof Error ? e.message : String(e)}`)
    }
    finally {
      isSavingRef.current = false
    }
  }

  const handleDelete = async (index: number) => {
    if (isSavingRef.current) {
      toast.error(t('placeholders.waitSaving'))
      return
    }

    const currentConfig = useConfigStore.getState().config
    if (!currentConfig) {
      return
    }

    const targetPlaceholder = currentConfig.placeholders[index]
    if (!targetPlaceholder) {
      return
    }

    const previousConfig = structuredClone(currentConfig)
    const nextPlaceholders = currentConfig.placeholders.filter((_, i) => i !== index)
    const nextRemovedNames = isSystemPlaceholder(targetPlaceholder)
      ? Array.from(new Set([
          ...(currentConfig.systemRemovedPlaceholderNames ?? []),
          targetPlaceholder.systemBaseName ?? targetPlaceholder.name,
        ]))
      : (currentConfig.systemRemovedPlaceholderNames ?? [])

    isSavingRef.current = true
    replacePlaceholdersState(nextPlaceholders, nextRemovedNames)

    try {
      await saveNow()
      addConfigLog(`удален плейсхолдер "${targetPlaceholder.name}"`)
      toast.success(t('placeholders.placeholderDeleted'))
    }
    catch (e) {
      revertTo(previousConfig)
      toast.error(`Ошибка сохранения: ${e instanceof Error ? e.message : String(e)}`)
    }
    finally {
      isSavingRef.current = false
    }
  }

  const handleRestorePlaceholder = async () => {
    if (!systemPlaceholderTarget || isSavingRef.current) {
      return
    }

    const currentConfig = useConfigStore.getState().config
    if (!currentConfig) {
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

    const targetName = systemPlaceholderTarget.name
    const previousConfig = structuredClone(currentConfig)
    const nextPlaceholders = currentConfig.placeholders.map((placeholder) => {
      if (placeholder.name !== targetName) {
        return placeholder
      }
      return buildRestoredPlaceholder(builtinPlaceholder)
    })

    isSavingRef.current = true
    replacePlaceholdersState(nextPlaceholders, currentConfig.systemRemovedPlaceholderNames)

    try {
      await saveNow()
      addConfigLog(`плейсхолдер "${targetName}" обновлен до системного значения`)
      toast.success(t('placeholders.placeholderUpdated'))
    }
    catch (e) {
      revertTo(previousConfig)
      toast.error(`Ошибка сохранения: ${e instanceof Error ? e.message : String(e)}`)
    }
    finally {
      isSavingRef.current = false
      setSystemPlaceholderTarget(null)
    }
  }

  const handleOpenAppDirectory = async () => {
    try {
      await tauri.openAppDirectory()
    }
    catch (err) {
      console.error('Failed to open app directory:', err)
      toast.error('Не удалось открыть папку приложения')
    }
  }

  const placeholders = config?.placeholders ?? []

  if (loading) {
    return (
      <div className="flex h-full items-center justify-center">
        <Loader2 className="size-6 animate-spin" />
      </div>
    )
  }

  return (
    <div className="relative h-full min-h-0">
      <LenisScrollArea ref={scrollAreaRef} className="h-full min-h-0">
        <div className="space-y-6 p-6">
          <div className="flex items-center justify-between">
            <div>
              <h1 className="text-2xl font-medium">{t('placeholders.title')}</h1>
              <p className="text-muted-foreground mt-1 text-sm">
                {t('placeholders.subtitle')}
              </p>
            </div>
            <div className="flex items-center gap-2">
              <Button
                variant="outline"
                size="icon"
                onClick={() => void handleOpenAppDirectory()}
                title={t('placeholders.openAppDir')}
                aria-label={t('placeholders.openAppDir')}
              >
                <FolderOpen className="size-4" />
              </Button>
              <Button onClick={() => setAddOpen(true)} className="flex items-center gap-2">
                <Plus className="size-4" />
                {t('placeholders.newPlaceholder')}
              </Button>
            </div>
          </div>

          <div className="space-y-3">
            {placeholders.length === 0
              ? (
                  <p className="text-muted-foreground text-sm">{t('placeholders.noPlaceholders')}</p>
                )
              : (
                  placeholders.map((placeholder, index) => {
                    const isSystem = isSystemPlaceholder(placeholder)
                    const isModified = isSystemPlaceholderModified(placeholder)
                    const builtinPlaceholder = getBuiltinPlaceholder(
                      builtinConfig,
                      placeholder.name,
                      placeholder.systemBaseName,
                    )
                    const hasUpdate = isSystemPlaceholderUpdateAvailable(placeholder, builtinPlaceholder)

                    return (
                      <div
                        key={placeholder.name}
                        className="bg-card flex min-h-[4.5rem] items-center justify-between gap-4 overflow-hidden rounded-lg border px-4 py-3"
                      >
                        <div className="flex min-w-0 w-0 flex-1 items-center gap-3 overflow-hidden">
                          <div className="flex size-9 shrink-0 items-center justify-center rounded-md border border-border/70 bg-muted/25 text-muted-foreground">
                            <FileCode className="size-4 text-[#DA702C] dark:text-[#DA702C]" />
                          </div>
                          <div className="min-w-0 w-0 flex-1 overflow-hidden space-y-1">
                            <div className="flex items-center gap-1 truncate text-sm font-normal text-foreground">
                              {'{{'}
                              {placeholder.name}
                              {'}}'}
                              <div className="flex items-center gap-1 text-muted-foreground">
                                {isSystem
                                  ? <InlineMarker icon={Package} label={t('placeholders.systemBadge')} />
                                  : <InlineMarker icon={UserRoundPlus} label={t('placeholders.customBadge')} className="text-primary/80" />}
                                {isModified && (
                                  <InlineMarker icon={FilePenLine} label={t('placeholders.modifiedBadge')} className="text-warning" />
                                )}
                                {isSystem && (isModified || hasUpdate) && (
                                  <InlineMarker
                                    icon={hasUpdate ? RefreshCcw : RotateCcw}
                                    label={hasUpdate
                                      ? t('placeholders.updateAvailable')
                                      : t('placeholders.rollbackToSystem')}
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
                            aria-label={t('placeholders.editAria', { name: placeholder.name })}
                            onClick={() => handleEdit(index, placeholder)}
                          >
                            <Pencil className="size-4" />
                          </Button>
                          <Button
                            variant="outline"
                            size="icon"
                            className="bg-destructive/10 text-destructive hover:bg-destructive/18"
                            aria-label={t('placeholders.deleteAria', { name: placeholder.name })}
                            onClick={() => handleDelete(index)}
                          >
                            <Trash2 className="size-4" />
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
                <DialogTitle>{t('placeholders.newPlaceholder')}</DialogTitle>
              </DialogHeader>
              <div className="space-y-4 py-4">
                <Input
                  aria-label={t('placeholders.nameInputAria')}
                  placeholder={t('placeholders.namePlaceholder')}
                  value={newName}
                  onChange={e => setNewName(e.target.value)}
                />
                <Input
                  aria-label={t('placeholders.pathInputAria')}
                  placeholder={t('placeholders.pathPlaceholder')}
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
                  {t('common.cancel')}
                </Button>
                <Button onClick={handleAdd}>{t('placeholders.addButton')}</Button>
              </DialogFooter>
            </DialogContent>
          </Dialog>

          <Dialog open={editingIndex !== null} onOpenChange={open => !open && setEditingIndex(null)}>
            <DialogContent>
              <DialogHeader>
                <DialogTitle>{t('placeholders.editPlaceholder')}</DialogTitle>
              </DialogHeader>
              <div className="space-y-4 py-4">
                <div className="space-y-2">
                  <Label htmlFor="edit-placeholder-name">{t('placeholders.nameLabel')}</Label>
                  <Input
                    id="edit-placeholder-name"
                    aria-label={t('placeholders.nameInputAria')}
                    placeholder={t('placeholders.namePlaceholder')}
                    value={editName}
                    onChange={e => setEditName(e.target.value)}
                  />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="edit-placeholder-path">{t('placeholders.pathLabel')}</Label>
                  <Input
                    id="edit-placeholder-path"
                    aria-label={t('placeholders.pathInputAria')}
                    placeholder={t('placeholders.pathPlaceholder')}
                    value={editPath}
                    onChange={e => setEditPath(e.target.value)}
                  />
                </div>
              </div>
              <DialogFooter>
                <Button variant="outline" onClick={() => setEditingIndex(null)}>
                  {t('common.cancel')}
                </Button>
                <Button onClick={handleSaveEdit}>{t('common.save')}</Button>
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
                    ? t('placeholders.updateSystemDialogTitle')
                    : t('placeholders.restoreSystemDialogTitle')}
                </AlertDialogTitle>
                <AlertDialogDescription>
                  {systemPlaceholderTarget
                    ? t('placeholders.updateSystemDialogDescription', { name: systemPlaceholderTarget.name })
                    : ''}
                </AlertDialogDescription>
              </AlertDialogHeader>
              <AlertDialogFooter>
                <AlertDialogCancel>{t('common.cancel')}</AlertDialogCancel>
                <AlertDialogAction onClick={() => void handleRestorePlaceholder()}>
                  {t('placeholders.updateDialogConfirm')}
                </AlertDialogAction>
              </AlertDialogFooter>
            </AlertDialogContent>
          </AlertDialog>
        </div>
      </LenisScrollArea>
      <ScrollTopButton scrollAreaRef={scrollAreaRef} />
    </div>
  )
}
