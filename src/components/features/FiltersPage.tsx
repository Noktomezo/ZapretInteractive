import type { Filter as FilterType } from '@/lib/types'
import { FilePenLine, Filter, FolderOpen, Loader2, Package, Pencil, Plus, RefreshCcw, RotateCcw, Trash2, UserRoundPlus } from 'lucide-react'
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
  AlertDialogTrigger,
} from '@/components/ui/alert-dialog'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { EditorTextarea } from '@/components/ui/editor-textarea'
import { InlineMarker } from '@/components/ui/inline-marker'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { LenisScrollArea } from '@/components/ui/lenis-scroll-area'
import { ScrollTopButton } from '@/components/ui/scroll-top-button'
import { Switch } from '@/components/ui/switch'
import { useMountEffect } from '@/hooks/use-mount-effect'
import { autosizeTextarea } from '@/lib/editor-scroll'
import { buildRestoredFilter, getBuiltinFilter, isSystemFilter, isSystemFilterModified, isSystemFilterUpdateAvailable } from '@/lib/system-config'
import * as tauri from '@/lib/tauri'
import { useConfigStore } from '@/stores/config.store'
import { useConnectionStore } from '@/stores/connection.store'

interface FilterDraft {
  name: string
  filename: string
  content: string
}

const TRAILING_SLASHES = /[/\\]+$/
const PATH_SEGMENT_SEPARATOR = /[/\\]+/
const arrayAt = Array.prototype as { at?: (this: string[], index: number) => string | undefined }

function getPathLeaf(path: string) {
  const normalizedPath = path.trim().replace(TRAILING_SLASHES, '')
  if (!normalizedPath) {
    return path.trim()
  }

  const segments = normalizedPath.split(PATH_SEGMENT_SEPARATOR)
  return arrayAt.at?.call(segments, -1) ?? normalizedPath
}

function normalizeFilterFilename(filename: string) {
  return getPathLeaf(filename.trim())
}

const emptyDraft: FilterDraft = {
  name: '',
  filename: '',
  content: '',
}

export function FiltersPage() {
  const { t } = useTranslation()
  const config = useConfigStore(state => state.config)
  const builtinConfig = useConfigStore(state => state.builtinConfig)
  const loading = useConfigStore(state => state.loading)
  const load = useConfigStore(state => state.load)
  const setFilters = useConfigStore(state => state.setFilters)
  const replaceFiltersState = useConfigStore(state => state.replaceFiltersState)
  const saveNow = useConfigStore(state => state.saveNow)
  const restartIfConnected = useConnectionStore(state => state.restartIfConnected)
  const notifyConfigApplied = useConnectionStore(state => state.notifyConfigApplied)
  const addConfigLog = useConnectionStore(state => state.addConfigLog)
  const [createDialogOpen, setCreateDialogOpen] = useState(false)
  const [editDialogOpen, setEditDialogOpen] = useState(false)
  const editingFilterIdRef = useRef<string | null>(null)
  const scrollAreaRef = useRef<HTMLDivElement | null>(null)
  const [draft, setDraft] = useState<FilterDraft>(emptyDraft)
  const [editLoading, setEditLoading] = useState(false)
  const [editLoadSucceeded, setEditLoadSucceeded] = useState(false)
  const [currentLoadId, setCurrentLoadId] = useState<string | null>(null)
  const currentLoadIdRef = useRef<string | null>(null)
  const [createInFlight, setCreateInFlight] = useState(false)
  const [editInFlight, setEditInFlight] = useState(false)
  const [deleteInFlightId, setDeleteInFlightId] = useState<string | null>(null)
  const reservedBundledFilenamesRef = useRef<Set<string>>(new Set())
  const [systemFilterTarget, setSystemFilterTarget] = useState<FilterType | null>(null)
  const latestMutationIdRef = useRef(0)
  const createContentTextareaRef = useRef<HTMLTextAreaElement | null>(null)
  const editContentTextareaRef = useRef<HTMLTextAreaElement | null>(null)

  useMountEffect(() => {
    Promise.all([
      load(),
      tauri.getReservedFilterFilenames().catch(() => []),
    ]).then(([_, bundledFiles]) => {
      reservedBundledFilenamesRef.current = new Set(bundledFiles.map(normalizeFilterFilename))
    }).catch(console.error)
  })

  const resetDraft = () => {
    setDraft(emptyDraft)
    editingFilterIdRef.current = null
    currentLoadIdRef.current = null
    setCurrentLoadId(null)
    setEditLoading(false)
    setEditLoadSucceeded(false)
  }

  const updateDraft = (patch: Partial<FilterDraft>) => {
    setDraft(prev => ({ ...prev, ...patch }))
  }

  const validateFilename = (filename: string, currentFilter?: FilterType) => {
    const nextFilename = normalizeFilterFilename(filename)
    if (!nextFilename) {
      toast.error('Имя файла не может быть пустым')
      return false
    }

    const currentFilters = useConfigStore.getState().config?.filters || []
    const duplicateInConfig = currentFilters.some(
      filter => filter.id !== currentFilter?.id && normalizeFilterFilename(filter.filename).toLowerCase() === nextFilename.toLowerCase(),
    )

    if (duplicateInConfig) {
      toast.error(t('filters.fileAlreadyExists'))
      return false
    }

    const isCurrentFile = currentFilter && normalizeFilterFilename(currentFilter.filename).toLowerCase() === nextFilename.toLowerCase()
    if (!isCurrentFile && reservedBundledFilenamesRef.current.has(nextFilename)) {
      toast.error(t('filters.fileAlreadyExists'))
      return false
    }

    return true
  }

  const validateFilterDraft = (nextDraft: FilterDraft, currentFilter?: FilterType) => {
    const nextName = nextDraft.name.trim().toLocaleLowerCase()
    const nextContent = nextDraft.content.trim()
    const currentFilters = useConfigStore.getState().config?.filters || []

    if (currentFilters.some(filter => filter.id !== currentFilter?.id && filter.name.trim().toLocaleLowerCase() === nextName)) {
      toast.error(t('filters.nameExists'))
      return false
    }

    if (nextContent && currentFilters.some(filter => filter.id !== currentFilter?.id && filter.content.trim() === nextContent)) {
      toast.error(t('filters.contentExists'))
      return false
    }

    return true
  }

  const persistFilters = async (nextFilters: FilterType[], previousFilters: FilterType[]) => {
    const mutationId = ++latestMutationIdRef.current
    setFilters(nextFilters)
    try {
      await saveNow()
    }
    catch (e) {
      if (latestMutationIdRef.current === mutationId) {
        setFilters(previousFilters)
      }
      throw e
    }
  }

  const handleToggleFilter = (filterId: string) => {
    const currentFilters = useConfigStore.getState().config?.filters || []
    const targetFilter = currentFilters.find(filter => filter.id === filterId)
    const updatedFilters = currentFilters.map(filter =>
      filter.id === filterId ? { ...filter, active: !filter.active } : filter,
    )
    void persistFilters(updatedFilters, currentFilters)
      .then(() => {
        if (targetFilter) {
          addConfigLog(`фильтр "${targetFilter.name}" ${targetFilter.active ? 'отключён' : 'включён'}`)
        }
        return restartIfConnected()
          .then(() => {
            notifyConfigApplied(t('filters.filterUpdated'))
          })
          .catch((e) => {
            toast.error(`Ошибка применения фильтров: ${e instanceof Error ? e.message : String(e)}`)
          })
      })
      .catch((e) => {
        toast.error(`Ошибка сохранения фильтров: ${e instanceof Error ? e.message : String(e)}`)
      })
  }

  const handleCreateFilter = async () => {
    if (createInFlight)
      return
    if (!draft.name.trim() || !draft.filename.trim())
      return

    const nextFilename = normalizeFilterFilename(draft.filename)
    if (!validateFilename(nextFilename))
      return
    if (!validateFilterDraft(draft))
      return

    setCreateInFlight(true)
    try {
      const newFilter: FilterType = {
        id: `filter-${crypto.randomUUID()}`,
        name: draft.name.trim(),
        filename: nextFilename,
        active: true,
        content: draft.content ?? '',
      }

      await tauri.saveFilterFile(nextFilename, draft.content ?? '')

      const currentFilters = useConfigStore.getState().config?.filters || []
      await persistFilters([...currentFilters, newFilter], currentFilters)
      addConfigLog(`добавлен фильтр "${newFilter.name}" (${newFilter.filename})`)
      resetDraft()
      setCreateDialogOpen(false)
      toast.success(t('filters.filterCreated'))
    }
    catch (e) {
      await tauri.deleteFilterFile(nextFilename).catch(() => {})
      toast.error(`Ошибка создания фильтра: ${e instanceof Error ? e.message : String(e)}`)
    }
    finally {
      setCreateInFlight(false)
    }
  }

  const openEditDialog = async (filter: FilterType) => {
    const loadId = crypto.randomUUID()
    currentLoadIdRef.current = loadId
    setCurrentLoadId(loadId)
    editingFilterIdRef.current = filter.id
    setDraft({
      name: filter.name,
      filename: normalizeFilterFilename(filter.filename),
      content: filter.content ?? '',
    })
    setEditLoading(true)
    setEditLoadSucceeded(false)
    setEditDialogOpen(true)

    try {
      const content = await tauri.loadFilterFile(filter.filename)
      if (currentLoadIdRef.current === loadId) {
        setDraft({
          name: filter.name,
          filename: normalizeFilterFilename(filter.filename),
          content,
        })
        setEditLoadSucceeded(true)
      }
    }
    catch (e) {
      if (currentLoadIdRef.current === loadId) {
        toast.error(`Ошибка загрузки содержимого фильтра: ${e instanceof Error ? e.message : String(e)}`)
        setEditLoadSucceeded(false)
      }
    }
    finally {
      if (currentLoadIdRef.current === loadId) {
        setEditLoading(false)
        setCurrentLoadId(null)
      }
    }
  }

  const handleSaveEdit = async () => {
    const currentFilters = useConfigStore.getState().config?.filters || []
    const currentFilter = currentFilters.find(filter => filter.id === editingFilterIdRef.current)
    if (!currentFilter || editInFlight || !editLoadSucceeded)
      return
    if (!draft.name.trim() || !draft.filename.trim())
      return

    const nextFilename = normalizeFilterFilename(draft.filename)
    if (!validateFilename(nextFilename, currentFilter))
      return
    if (!validateFilterDraft(draft, currentFilter))
      return

    const prevFilename = normalizeFilterFilename(currentFilter.filename)
    const isFilenameChanged = prevFilename.toLowerCase() !== nextFilename.toLowerCase()
    let prevContent: string | null = null

    setEditInFlight(true)
    try {
      if (isFilenameChanged) {
        prevContent = await tauri.loadFilterFile(prevFilename).catch(() => currentFilter.content)
        await tauri.saveFilterFile(nextFilename, draft.content ?? '')
        await tauri.deleteFilterFile(prevFilename)
      }
      else {
        await tauri.saveFilterFile(nextFilename, draft.content ?? '')
      }

      const updatedFilters = currentFilters.map((filter) => {
        if (filter.id !== currentFilter.id)
          return filter
        return {
          ...filter,
          name: draft.name.trim(),
          filename: nextFilename,
          content: draft.content ?? '',
        }
      })

      await persistFilters(updatedFilters, currentFilters)
      if (currentFilter.active) {
        await restartIfConnected()
        notifyConfigApplied(t('filters.filterUpdated'))
      }

      addConfigLog(`фильтр "${draft.name.trim()}" (${nextFilename}) сохранён`)
      setEditDialogOpen(false)
      resetDraft()
      toast.success(t('filters.filterSaved'))
    }
    catch (e) {
      if (isFilenameChanged && prevContent !== null) {
        await tauri.saveFilterFile(prevFilename, prevContent).catch(() => {})
        await tauri.deleteFilterFile(nextFilename).catch(() => {})
      }
      toast.error(`Ошибка сохранения фильтра: ${e instanceof Error ? e.message : String(e)}`)
    }
    finally {
      setEditInFlight(false)
    }
  }

  const handleDeleteFilter = async (filter: FilterType) => {
    if (deleteInFlightId)
      return
    const currentFilters = config?.filters || []
    const filename = normalizeFilterFilename(filter.filename)
    const nextFilters = currentFilters.filter(f => f.id !== filter.id)
    const nextRemovedFilterIds = filter.system
      ? Array.from(new Set([...(config?.systemRemovedFilterIds ?? []), filter.systemBaseName ?? filter.id]))
      : (config?.systemRemovedFilterIds ?? [])

    setDeleteInFlightId(filter.id)
    let fileContent: string | null = null

    try {
      fileContent = await tauri.loadFilterFile(filename).catch(() => filter.content)
      await tauri.deleteFilterFile(filename)
      replaceFiltersState(nextFilters, nextRemovedFilterIds)
      await saveNow()

      if (filter.active) {
        await restartIfConnected()
        notifyConfigApplied(t('filters.filterUpdated'))
      }

      addConfigLog(`фильтр "${filter.name}" (${filter.filename}) удалён`)
      toast.success(t('filters.filterDeleted'))
    }
    catch (e) {
      if (fileContent !== null) {
        await tauri.saveFilterFile(filename, fileContent).catch(() => {})
      }
      replaceFiltersState(currentFilters, config?.systemRemovedFilterIds ?? [])
      toast.error(`Ошибка удаления фильтра: ${e instanceof Error ? e.message : String(e)}`)
    }
    finally {
      setDeleteInFlightId(null)
    }
  }

  const handleRestoreFilter = async () => {
    if (!systemFilterTarget || !config) {
      return
    }

    const builtinFilter = getBuiltinFilter(builtinConfig, systemFilterTarget.id)
    if (!builtinFilter) {
      return
    }

    const previousFilters = structuredClone(config.filters)
    const nextFilters = config.filters.map(filter =>
      filter.id === systemFilterTarget.id ? buildRestoredFilter(filter, builtinFilter) : filter,
    )
    const nextRemovedFilterIds = (config.systemRemovedFilterIds ?? []).filter(id => id !== builtinFilter.id)
    const originalFilename = normalizeFilterFilename(systemFilterTarget.filename)
    const nextFilename = normalizeFilterFilename(builtinFilter.filename)
    const isCaseInsensitiveSameFile = originalFilename.toLowerCase() === nextFilename.toLowerCase()
    const originalContent = await tauri.loadFilterFile(originalFilename).catch(() => systemFilterTarget.content)
    let wroteNextFile = false
    let deletedOriginalFile = false

    try {
      await tauri.saveFilterFile(nextFilename, builtinFilter.content)
      wroteNextFile = true
      if (!isCaseInsensitiveSameFile && originalFilename !== nextFilename) {
        await tauri.deleteFilterFile(originalFilename)
        deletedOriginalFile = true
      }

      replaceFiltersState(nextFilters, nextRemovedFilterIds)
      await saveNow()
    }
    catch (error) {
      if (deletedOriginalFile) {
        await tauri.saveFilterFile(originalFilename, originalContent).catch(() => {})
      }
      else if (wroteNextFile && isCaseInsensitiveSameFile) {
        await tauri.saveFilterFile(originalFilename, originalContent).catch(() => {})
      }

      if (wroteNextFile && !isCaseInsensitiveSameFile && originalFilename !== nextFilename) {
        await tauri.deleteFilterFile(nextFilename).catch(() => {})
      }

      replaceFiltersState(previousFilters, config.systemRemovedFilterIds ?? [])
      toast.error(`Ошибка обновления фильтра: ${error instanceof Error ? error.message : String(error)}`)
      return
    }

    try {
      addConfigLog(`фильтр "${systemFilterTarget.name}" обновлён до системного значения`)
      await restartIfConnected()
      notifyConfigApplied(t('filters.filterUpdated'))
      setSystemFilterTarget(null)
    }
    catch (error) {
      toast.error(`Фильтр обновлён, но не удалось применить изменения: ${error instanceof Error ? error.message : String(error)}`)
    }
  }

  if (loading || !config) {
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
              <h1 className="text-2xl font-medium">{t('filters.title')}</h1>
              <p className="mt-1 text-sm text-muted-foreground">
                {t('filters.subtitle')}
              </p>
            </div>
            <div className="flex items-center gap-1">
              <Button onClick={() => setCreateDialogOpen(true)}>
                <Plus className="size-4" />
                {t('filters.newFilter')}
              </Button>
              <Button
                variant="outline"
                size="icon"
                title={t('filters.openFiltersFolder')}
                aria-label={t('filters.openFiltersFolder')}
                onClick={async () => {
                  try {
                    await tauri.openFiltersDirectory()
                  }
                  catch (e) {
                    toast.error(`Ошибка открытия папки фильтров: ${e instanceof Error ? e.message : String(e)}`)
                  }
                }}
              >
                <FolderOpen className="size-4" />
              </Button>
            </div>
          </div>

          <div className="space-y-3">
            {config.filters?.map((filter: FilterType) => {
              const builtin = getBuiltinFilter(builtinConfig, filter.id)
              const hasUpdate = isSystemFilterUpdateAvailable(filter, builtin)

              return (
                <div
                  key={filter.id}
                  className="flex min-h-[4.5rem] items-center justify-between overflow-hidden rounded-lg border bg-card px-4 py-3"
                >
                  <div className="flex min-w-0 w-0 flex-1 items-center gap-3 overflow-hidden">
                    <div className="flex size-9 shrink-0 items-center justify-center rounded-md border border-border/70 bg-muted/25 text-muted-foreground">
                      <Filter className="size-4 text-[#8B7EC8] dark:text-[#8B7EC8]" />
                    </div>
                    <div className="min-w-0 w-0 flex-1 overflow-hidden space-y-1">
                      <div className="flex items-center gap-1">
                        <Label htmlFor={filter.id} className="block cursor-pointer truncate text-sm font-normal">
                          {filter.name}
                        </Label>
                        <div className="flex items-center gap-1 text-muted-foreground">
                          {isSystemFilter(filter)
                            ? <InlineMarker icon={Package} label={t('filters.systemBadge')} />
                            : <InlineMarker icon={UserRoundPlus} label={t('filters.customBadge')} className="text-primary/80" />}
                          {isSystemFilterModified(filter) && (
                            <InlineMarker icon={FilePenLine} label={t('filters.modifiedBadge')} className="text-warning" />
                          )}
                          {isSystemFilter(filter) && (isSystemFilterModified(filter) || hasUpdate) && (
                            <InlineMarker
                              icon={hasUpdate ? RefreshCcw : RotateCcw}
                              label={hasUpdate
                                ? t('filters.updateAvailable')
                                : t('filters.rollbackToSystem')}
                              className={hasUpdate ? 'text-primary' : 'text-destructive'}
                              onClick={() => setSystemFilterTarget(filter)}
                            />
                          )}
                        </div>
                      </div>
                      <p className="truncate overflow-hidden text-xs text-muted-foreground/90" title={getPathLeaf(filter.filename)}>
                        {getPathLeaf(filter.filename)}
                      </p>
                    </div>
                  </div>
                  <div className="flex items-center gap-1">
                    <Switch
                      id={filter.id}
                      checked={filter.active}
                      onCheckedChange={() => handleToggleFilter(filter.id)}
                    />
                    <Button
                      variant="outline"
                      size="icon"
                      aria-label={t('filters.editAria', { name: filter.name })}
                      title={t('filters.editAria', { name: filter.name })}
                      disabled={editInFlight || deleteInFlightId === filter.id}
                      onClick={() => openEditDialog(filter)}
                    >
                      <Pencil className="size-4" />
                    </Button>
                    <AlertDialog>
                      <AlertDialogTrigger asChild>
                        <Button
                          variant="outline"
                          size="icon"
                          className="bg-destructive/10 text-destructive hover:bg-destructive/18"
                          aria-label={t('filters.deleteAria', { name: filter.name })}
                          title={t('filters.deleteAria', { name: filter.name })}
                          disabled={deleteInFlightId === filter.id || editInFlight}
                        >
                          <Trash2 className="size-4" />
                        </Button>
                      </AlertDialogTrigger>
                      <AlertDialogContent>
                        <AlertDialogHeader>
                          <AlertDialogTitle>{t('filters.deleteDialogTitle')}</AlertDialogTitle>
                          <AlertDialogDescription>
                            {t('filters.deleteDialogDescription', { name: filter.name, filename: filter.filename })}
                          </AlertDialogDescription>
                        </AlertDialogHeader>
                        <AlertDialogFooter>
                          <AlertDialogCancel>{t('common.cancel')}</AlertDialogCancel>
                          <AlertDialogAction
                            onClick={() => handleDeleteFilter(filter)}
                            className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
                          >
                            {t('common.delete')}
                          </AlertDialogAction>
                        </AlertDialogFooter>
                      </AlertDialogContent>
                    </AlertDialog>
                  </div>
                </div>
              )
            })}
          </div>

          <AlertDialog open={!!systemFilterTarget} onOpenChange={open => !open && setSystemFilterTarget(null)}>
            <AlertDialogContent>
              <AlertDialogHeader>
                <AlertDialogTitle>
                  {systemFilterTarget && isSystemFilterUpdateAvailable(systemFilterTarget, getBuiltinFilter(builtinConfig, systemFilterTarget.id))
                    ? t('filters.updateSystemDialogTitle')
                    : t('filters.restoreSystemDialogTitle')}
                </AlertDialogTitle>
                <AlertDialogDescription>
                  {systemFilterTarget
                    ? t('filters.updateSystemDialogDescription', { name: systemFilterTarget.name })
                    : ''}
                </AlertDialogDescription>
              </AlertDialogHeader>
              <AlertDialogFooter>
                <AlertDialogCancel>{t('common.cancel')}</AlertDialogCancel>
                <AlertDialogAction onClick={() => void handleRestoreFilter()}>
                  {t('filters.updateDialogConfirm')}
                </AlertDialogAction>
              </AlertDialogFooter>
            </AlertDialogContent>
          </AlertDialog>

          <Dialog
            open={createDialogOpen}
            onOpenChange={(open) => {
              setCreateDialogOpen(open)
              if (!open)
                resetDraft()
            }}
          >
            <DialogContent className="max-h-[calc(100vh-4rem)] max-w-2xl overflow-hidden">
              <DialogHeader>
                <DialogTitle>{t('filters.createDialogTitle')}</DialogTitle>
                <DialogDescription>
                  {t('filters.createDialogDescription')}
                </DialogDescription>
              </DialogHeader>
              <div className="space-y-4 py-4">
                <div className="space-y-2">
                  <Label htmlFor="filter-name">{t('filters.nameLabel')}</Label>
                  <Input
                    id="filter-name"
                    value={draft.name}
                    onChange={e => updateDraft({ name: e.target.value })}
                    placeholder="Discord Media"
                  />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="filter-filename">{t('filters.filenameLabel')}</Label>
                  <Input
                    id="filter-filename"
                    value={draft.filename}
                    onChange={e => updateDraft({ filename: e.target.value })}
                    placeholder="my-filter.txt"
                  />
                  {draft.filename.trim() && (
                    <p className="text-xs text-muted-foreground break-all">
                      {getPathLeaf(draft.filename.trim())}
                    </p>
                  )}
                </div>
                <div className="space-y-2">
                  <Label htmlFor="filter-content">{t('filters.contentLabel')}</Label>
                  <EditorTextarea
                    textareaRef={createContentTextareaRef}
                    id="filter-content"
                    value={draft.content}
                    onChange={(e) => {
                      updateDraft({ content: e.target.value })
                      autosizeTextarea(e.currentTarget)
                    }}
                    placeholder={t('filters.contentPlaceholder')}
                    rows={10}
                  />
                </div>
              </div>
              <DialogFooter>
                <Button variant="outline" onClick={() => setCreateDialogOpen(false)}>
                  {t('common.cancel')}
                </Button>
                <Button onClick={handleCreateFilter} disabled={createInFlight || !draft.name.trim() || !draft.filename.trim()}>
                  {t('filters.createButton')}
                </Button>
              </DialogFooter>
            </DialogContent>
          </Dialog>

          <Dialog
            open={editDialogOpen}
            onOpenChange={(open) => {
              setEditDialogOpen(open)
              if (!open)
                resetDraft()
            }}
          >
            <DialogContent className="max-h-[calc(100vh-4rem)] max-w-3xl overflow-hidden">
              <DialogHeader>
                <DialogTitle>{t('filters.editDialogTitle')}</DialogTitle>
                <DialogDescription>
                  {t('filters.editDialogDescription')}
                </DialogDescription>
              </DialogHeader>
              <div className="space-y-4 py-4">
                <div className="grid grid-cols-2 gap-4">
                  <div className="space-y-2">
                    <Label htmlFor="edit-filter-name">{t('filters.nameLabel')}</Label>
                    <Input
                      id="edit-filter-name"
                      value={draft.name}
                      onChange={e => updateDraft({ name: e.target.value })}
                      placeholder="Discord Media"
                      disabled={editLoading}
                    />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="edit-filter-filename">{t('filters.filenameLabel')}</Label>
                    <Input
                      id="edit-filter-filename"
                      value={draft.filename}
                      onChange={e => updateDraft({ filename: e.target.value })}
                      placeholder="my-filter.txt"
                      disabled={editLoading}
                    />
                  </div>
                </div>
                <div className="space-y-2">
                  <Label htmlFor="edit-filter-content">{t('filters.contentLabel')}</Label>
                  <EditorTextarea
                    textareaRef={editContentTextareaRef}
                    id="edit-filter-content"
                    value={draft.content}
                    onChange={(e) => {
                      updateDraft({ content: e.target.value })
                      autosizeTextarea(e.currentTarget)
                    }}
                    placeholder={t('filters.contentPlaceholder')}
                    rows={16}
                    disabled={editLoading}
                  />
                  {editLoading && currentLoadId && (
                    <p className="text-xs text-muted-foreground">{t('filters.loadingContent')}</p>
                  )}
                </div>
              </div>
              <DialogFooter>
                <Button variant="outline" onClick={() => setEditDialogOpen(false)}>
                  {t('filters.closeDialog')}
                </Button>
                <Button onClick={handleSaveEdit} disabled={editLoading || editInFlight || !editLoadSucceeded || !draft.name.trim() || !draft.filename.trim()}>
                  {t('common.save')}
                </Button>
              </DialogFooter>
            </DialogContent>
          </Dialog>
        </div>
      </LenisScrollArea>
      <ScrollTopButton scrollAreaRef={scrollAreaRef} />
    </div>
  )
}
