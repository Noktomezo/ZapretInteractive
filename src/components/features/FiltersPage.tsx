import type { Filter as FilterType } from '@/lib/types'
import { FilePenLine, Filter, FolderOpen, Loader2, Package, Pencil, Plus, RefreshCcw, RotateCcw, Trash2, UserRoundPlus } from 'lucide-react'
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
import { InlineMarker } from '@/components/ui/inline-marker'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { LenisScrollArea } from '@/components/ui/lenis-scroll-area'
import { Switch } from '@/components/ui/switch'
import { Textarea } from '@/components/ui/textarea'
import { useMountEffect } from '@/hooks/use-mount-effect'
import { autosizeTextarea, forwardTextareaWheelToScrollArea } from '@/lib/editor-scroll'
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
  const [editingFilterId, setEditingFilterId] = useState<string | null>(null)
  const [draft, setDraft] = useState<FilterDraft>(emptyDraft)
  const [editLoading, setEditLoading] = useState(false)
  const [editLoadSucceeded, setEditLoadSucceeded] = useState(false)
  const [currentLoadId, setCurrentLoadId] = useState<string | null>(null)
  const currentLoadIdRef = useRef<string | null>(null)
  const [createInFlight, setCreateInFlight] = useState(false)
  const [editInFlight, setEditInFlight] = useState(false)
  const [deleteInFlightId, setDeleteInFlightId] = useState<string | null>(null)
  const [reservedBundledFilenames, setReservedBundledFilenames] = useState<Set<string>>(new Set())
  const [systemFilterTarget, setSystemFilterTarget] = useState<FilterType | null>(null)
  const latestMutationIdRef = useRef(0)
  const createContentTextareaRef = useRef<HTMLTextAreaElement | null>(null)
  const editContentTextareaRef = useRef<HTMLTextAreaElement | null>(null)

  useMountEffect(() => {
    Promise.all([
      load(),
      tauri.getReservedFilterFilenames().then((names) => {
        setReservedBundledFilenames(new Set(names.map(name => name.trim().toLowerCase())))
      }),
    ]).catch(console.error)
  })

  const resetDraft = () => {
    setDraft(emptyDraft)
    setEditingFilterId(null)
    setEditLoading(false)
    setEditLoadSucceeded(false)
    setCurrentLoadId(null)
    currentLoadIdRef.current = null
    requestAnimationFrame(() => {
      autosizeTextarea(createContentTextareaRef.current)
      autosizeTextarea(editContentTextareaRef.current)
    })
  }

  const updateDraft = (updates: Partial<FilterDraft>) => {
    setDraft(prev => ({ ...prev, ...updates }))
  }

  const validateFilename = (filename: string, currentFilename?: string) => {
    const normalized = normalizeFilterFilename(filename)
    const normalizedLower = normalized.toLowerCase()
    const currentFilenameLower = currentFilename ? normalizeFilterFilename(currentFilename).toLowerCase() : undefined
    if (!normalized) {
      toast.error('Укажите имя файла фильтра')
      return false
    }

    const existingFilters = useConfigStore.getState().config?.filters || []
    const hasCollision = existingFilters.some(filter =>
      normalizeFilterFilename(filter.filename).toLowerCase() === normalizedLower
      && normalizeFilterFilename(filter.filename).toLowerCase() !== currentFilenameLower,
    )

    if (hasCollision) {
      toast.error('Фильтр с таким именем файла уже существует')
      return false
    }

    if (reservedBundledFilenames.has(normalizedLower) && normalizedLower !== currentFilenameLower) {
      toast.error('Это имя файла зарезервировано встроенным фильтром')
      return false
    }

    return true
  }

  const validateFilterDraft = (nextDraft: FilterDraft, currentFilter?: FilterType) => {
    const nextName = nextDraft.name.trim().toLocaleLowerCase()
    const nextContent = nextDraft.content.trim()
    const currentFilters = useConfigStore.getState().config?.filters || []

    if (currentFilters.some(filter => filter.id !== currentFilter?.id && filter.name.trim().toLocaleLowerCase() === nextName)) {
      toast.error('Фильтр с таким названием уже существует')
      return false
    }

    if (nextContent && currentFilters.some(filter => filter.id !== currentFilter?.id && filter.content.trim() === nextContent)) {
      toast.error('Фильтр с таким содержимым уже существует')
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
            notifyConfigApplied('Фильтр обновлён')
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
      toast.success('Фильтр создан')
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
    setEditingFilterId(filter.id)
    setEditDialogOpen(true)
    setEditLoading(true)
    setEditLoadSucceeded(false)
    setDraft({
      name: filter.name,
      filename: normalizeFilterFilename(filter.filename),
      content: filter.content,
    })

    try {
      const content = await tauri.loadFilterFile(normalizeFilterFilename(filter.filename))
      if (currentLoadIdRef.current !== loadId) {
        return
      }

      setDraft({
        name: filter.name,
        filename: normalizeFilterFilename(filter.filename),
        content,
      })
      requestAnimationFrame(() => autosizeTextarea(editContentTextareaRef.current))
      setEditLoadSucceeded(true)
    }
    catch (e) {
      if (currentLoadIdRef.current === loadId) {
        setEditLoadSucceeded(false)
        toast.error(`Ошибка загрузки фильтра: ${e instanceof Error ? e.message : String(e)}`)
      }
    }
    finally {
      if (currentLoadIdRef.current === loadId) {
        setEditLoading(false)
      }
    }
  }

  const handleSaveEdit = async () => {
    if (editInFlight)
      return
    if (!editingFilterId || !draft.name.trim() || !draft.filename.trim() || !editLoadSucceeded)
      return

    const currentFilters = useConfigStore.getState().config?.filters || []
    const targetFilter = currentFilters.find(filter => filter.id === editingFilterId)
    if (!targetFilter)
      return

    const nextFilename = normalizeFilterFilename(draft.filename)
    if (!validateFilename(nextFilename, targetFilter.filename))
      return
    if (!validateFilterDraft(draft, targetFilter))
      return

    const targetFilename = normalizeFilterFilename(targetFilter.filename)
    const renamed = targetFilename.toLowerCase() !== nextFilename.toLowerCase()
    setEditInFlight(true)
    const originalContent = await tauri.loadFilterFile(targetFilename).catch(() => draft.content)
    try {
      await tauri.saveFilterFile(nextFilename, draft.content)
      if (renamed) {
        try {
          await tauri.deleteFilterFile(targetFilename)
        }
        catch (e) {
          await tauri.deleteFilterFile(nextFilename).catch(() => {})
          throw e
        }
      }

      const updatedFilters = currentFilters.map(filter =>
        filter.id === editingFilterId
          ? {
              ...filter,
              name: draft.name.trim(),
              filename: nextFilename,
              content: draft.content,
            }
          : filter,
      )

      await persistFilters(updatedFilters, currentFilters)
      addConfigLog(
        renamed
          ? `фильтр "${targetFilter.name}" обновлён, файл переименован с ${targetFilename} на ${nextFilename}`
          : `обновлён фильтр "${draft.name.trim()}"`,
      )
      resetDraft()
      setEditDialogOpen(false)
      toast.success('Фильтр сохранён')
    }
    catch (e) {
      if (renamed) {
        await tauri.deleteFilterFile(nextFilename).catch(() => {})
      }
      await tauri.saveFilterFile(targetFilename, originalContent).catch(() => {})
      toast.error(`Ошибка сохранения фильтра: ${e instanceof Error ? e.message : String(e)}`)
    }
    finally {
      setEditInFlight(false)
    }
  }

  const handleDeleteFilter = async (filter: FilterType) => {
    if (deleteInFlightId)
      return
    setDeleteInFlightId(filter.id)
    let originalContent: string | undefined
    try {
      const filterFilename = normalizeFilterFilename(filter.filename)
      try {
        originalContent = await tauri.loadFilterFile(filterFilename)
      }
      catch {}
      await tauri.deleteFilterFile(filterFilename)
      const currentFilters = useConfigStore.getState().config?.filters || []
      const nextFilters = currentFilters.filter(item => item.id !== filter.id)
      const nextRemovedFilterIds = filter.system
        ? Array.from(new Set([...(config?.systemRemovedFilterIds ?? []), filter.id]))
        : (config?.systemRemovedFilterIds ?? [])
      try {
        replaceFiltersState(nextFilters, nextRemovedFilterIds)
        await saveNow()
      }
      catch (e) {
        if (originalContent !== undefined) {
          await tauri.saveFilterFile(filterFilename, originalContent).catch(() => {})
        }
        replaceFiltersState(currentFilters, config?.systemRemovedFilterIds ?? [])
        throw e
      }
      if (editingFilterId === filter.id) {
        resetDraft()
        setEditDialogOpen(false)
      }
      addConfigLog(`удалён фильтр "${filter.name}" (${filterFilename})`)
      toast.success('Фильтр удалён')
    }
    catch (e) {
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
      notifyConfigApplied('Фильтр обновлён')
      setSystemFilterTarget(null)
    }
    catch (error) {
      toast.error(`Фильтр обновлён, но не удалось применить изменения: ${error instanceof Error ? error.message : String(error)}`)
    }
  }

  if (loading || !config) {
    return (
      <div className="flex h-full items-center justify-center">
        <Loader2 className="h-6 w-6 animate-spin" />
      </div>
    )
  }

  return (
    <LenisScrollArea className="h-full min-h-0">
      <div className="space-y-6 p-6">
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-2xl font-medium">Фильтры</h1>
            <p className="mt-1 text-sm text-muted-foreground">
              WinDivert фильтры для отсечения полезной нагрузки
            </p>
          </div>
          <div className="flex items-center gap-1">
            <Button onClick={() => setCreateDialogOpen(true)}>
              <Plus className="h-4 w-4" />
              Новый фильтр
            </Button>
            <Button
              variant="outline"
              size="icon"
              title="Открыть папку filters"
              aria-label="Открыть папку filters"
              onClick={async () => {
                try {
                  await tauri.openFiltersDirectory()
                }
                catch (e) {
                  toast.error(`Ошибка открытия папки фильтров: ${e instanceof Error ? e.message : String(e)}`)
                }
              }}
            >
              <FolderOpen className="h-4 w-4" />
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
                  <div className="text-muted-foreground flex size-9 shrink-0 items-center justify-center rounded-md border border-border/70 bg-muted/25">
                    <Filter className="h-4 w-4" />
                  </div>
                  <div className="min-w-0 w-0 flex-1 overflow-hidden space-y-1">
                    <div className="flex items-center gap-2">
                      <Label htmlFor={filter.id} className="block cursor-pointer truncate text-sm font-normal">
                        {filter.name}
                      </Label>
                      <div className="flex items-center gap-1 text-muted-foreground">
                        {isSystemFilter(filter)
                          ? <InlineMarker icon={Package} label="Системный фильтр" />
                          : <InlineMarker icon={UserRoundPlus} label="Пользовательский фильтр" className="text-primary/80" />}
                        {isSystemFilterModified(filter) && (
                          <InlineMarker icon={FilePenLine} label="Системный фильтр изменён пользователем" className="text-warning" />
                        )}
                        {isSystemFilter(filter) && (isSystemFilterModified(filter) || hasUpdate) && (
                          <InlineMarker
                            icon={hasUpdate ? RefreshCcw : RotateCcw}
                            label={hasUpdate
                              ? 'Обновить фильтр до актуального системного значения'
                              : 'Откатить фильтр к системному значению'}
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
                    aria-label={`Редактировать фильтр ${filter.name}`}
                    title={`Редактировать фильтр ${filter.name}`}
                    disabled={editInFlight || deleteInFlightId === filter.id}
                    onClick={() => openEditDialog(filter)}
                  >
                    <Pencil className="h-4 w-4" />
                  </Button>
                  <AlertDialog>
                    <AlertDialogTrigger asChild>
                      <Button
                        variant="outline"
                        size="icon"
                        className="border-destructive/30 bg-destructive/10 text-destructive hover:bg-destructive/18"
                        aria-label={`Удалить фильтр ${filter.name}`}
                        title={`Удалить фильтр ${filter.name}`}
                        disabled={deleteInFlightId === filter.id || editInFlight}
                      >
                        <Trash2 className="h-4 w-4" />
                      </Button>
                    </AlertDialogTrigger>
                    <AlertDialogContent>
                      <AlertDialogHeader>
                        <AlertDialogTitle>Удалить фильтр?</AlertDialogTitle>
                        <AlertDialogDescription>
                          {`Фильтр "${filter.name}" будет удалён из списка, а файл ${filter.filename} будет удалён с диска.`}
                        </AlertDialogDescription>
                      </AlertDialogHeader>
                      <AlertDialogFooter>
                        <AlertDialogCancel>Отмена</AlertDialogCancel>
                        <AlertDialogAction
                          onClick={() => handleDeleteFilter(filter)}
                          className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
                        >
                          Удалить
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
                  ? 'Обновить системный фильтр?'
                  : 'Откатить фильтр к системному значению?'}
              </AlertDialogTitle>
              <AlertDialogDescription>
                {systemFilterTarget
                  ? `Фильтр "${systemFilterTarget.name}" будет возвращён к актуальному системному значению.`
                  : ''}
              </AlertDialogDescription>
            </AlertDialogHeader>
            <AlertDialogFooter>
              <AlertDialogCancel>Отмена</AlertDialogCancel>
              <AlertDialogAction onClick={() => void handleRestoreFilter()}>
                Обновить
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
              <DialogTitle>Новый фильтр</DialogTitle>
              <DialogDescription>
                Создайте новый файл фильтра и добавьте его в конфигурацию.
              </DialogDescription>
            </DialogHeader>
            <div className="space-y-4 py-4">
              <div className="space-y-2">
                <Label htmlFor="filter-name">Название</Label>
                <Input
                  id="filter-name"
                  value={draft.name}
                  onChange={e => updateDraft({ name: e.target.value })}
                  placeholder="Discord Media"
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="filter-filename">Имя файла</Label>
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
                <Label htmlFor="filter-content">Содержимое фильтра</Label>
                <LenisScrollArea
                  className="max-h-[calc(100vh-22rem)] rounded-md border border-border/80 bg-background/92 shadow-xs transition-[border-color,box-shadow,background-color] hover:border-border focus-within:border-ring focus-within:ring-[3px] focus-within:ring-ring/50 dark:bg-input/30"
                  contentClassName="cursor-text"
                  onClick={() => createContentTextareaRef.current?.focus()}
                >
                  <Textarea
                    data-lenis-prevent
                    ref={createContentTextareaRef}
                    id="filter-content"
                    value={draft.content}
                    onChange={(e) => {
                      updateDraft({ content: e.target.value })
                      autosizeTextarea(e.currentTarget)
                    }}
                    onWheel={forwardTextareaWheelToScrollArea}
                    placeholder="WinDivert фильтр..."
                    rows={10}
                    className="resize-none overflow-hidden rounded-none border-0 bg-transparent px-3 py-3 font-mono text-sm shadow-none hover:border-transparent focus-visible:border-transparent focus-visible:ring-0"
                  />
                </LenisScrollArea>
              </div>
            </div>
            <DialogFooter>
              <Button variant="outline" onClick={() => setCreateDialogOpen(false)}>
                Отмена
              </Button>
              <Button onClick={handleCreateFilter} disabled={createInFlight || !draft.name.trim() || !draft.filename.trim()}>
                Создать
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
              <DialogTitle>Редактировать фильтр</DialogTitle>
              <DialogDescription>
                Просмотр и изменение имени, файла и содержимого фильтра.
              </DialogDescription>
            </DialogHeader>
            <div className="space-y-4 py-4">
              <div className="grid grid-cols-2 gap-4">
                <div className="space-y-2">
                  <Label htmlFor="edit-filter-name">Название</Label>
                  <Input
                    id="edit-filter-name"
                    value={draft.name}
                    onChange={e => updateDraft({ name: e.target.value })}
                    placeholder="Discord Media"
                    disabled={editLoading}
                  />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="edit-filter-filename">Имя файла</Label>
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
                <Label htmlFor="edit-filter-content">Содержимое фильтра</Label>
                <LenisScrollArea
                  className="max-h-[calc(100vh-22rem)] rounded-md border border-border/80 bg-background/92 shadow-xs transition-[border-color,box-shadow,background-color] hover:border-border focus-within:border-ring focus-within:ring-[3px] focus-within:ring-ring/50 dark:bg-input/30"
                  contentClassName="cursor-text"
                  onClick={() => editContentTextareaRef.current?.focus()}
                >
                  <Textarea
                    data-lenis-prevent
                    ref={editContentTextareaRef}
                    id="edit-filter-content"
                    value={draft.content}
                    onChange={(e) => {
                      updateDraft({ content: e.target.value })
                      autosizeTextarea(e.currentTarget)
                    }}
                    onWheel={forwardTextareaWheelToScrollArea}
                    placeholder="WinDivert фильтр..."
                    rows={16}
                    className="resize-none overflow-hidden rounded-none border-0 bg-transparent px-3 py-3 font-mono text-sm shadow-none hover:border-transparent focus-visible:border-transparent focus-visible:ring-0"
                    disabled={editLoading}
                  />
                </LenisScrollArea>
                {editLoading && currentLoadId && (
                  <p className="text-xs text-muted-foreground">Загружаю содержимое файла...</p>
                )}
              </div>
            </div>
            <DialogFooter>
              <Button variant="outline" onClick={() => setEditDialogOpen(false)}>
                Закрыть
              </Button>
              <Button onClick={handleSaveEdit} disabled={editLoading || editInFlight || !editLoadSucceeded || !draft.name.trim() || !draft.filename.trim()}>
                Сохранить
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      </div>
    </LenisScrollArea>
  )
}
