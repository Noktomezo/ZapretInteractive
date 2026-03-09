import type { Filter as FilterType } from '@/lib/types'
import { openPath } from '@tauri-apps/plugin-opener'
import { Filter, FolderOpen, Loader2, Pencil, Plus, Trash2 } from 'lucide-react'
import { useEffect, useRef, useState } from 'react'
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
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Switch } from '@/components/ui/switch'
import { Textarea } from '@/components/ui/textarea'
import * as tauri from '@/lib/tauri'
import { useConfigStore } from '@/stores/config.store'
import { useConnectionStore } from '@/stores/connection.store'

interface FilterDraft {
  name: string
  filename: string
  content: string
}

const emptyDraft: FilterDraft = {
  name: '',
  filename: '',
  content: '',
}

export function FiltersPage() {
  const { config, loading, load, setFilters } = useConfigStore()
  const { restartIfConnected, notifyConfigApplied } = useConnectionStore()
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
  const isInitialLoadRef = useRef(true)
  const latestMutationIdRef = useRef(0)

  useEffect(() => {
    Promise.all([
      load().then(() => {
        isInitialLoadRef.current = false
      }),
      tauri.getReservedFilterFilenames().then((names) => {
        setReservedBundledFilenames(new Set(names.map(name => name.trim().toLowerCase())))
      }),
    ]).catch(console.error)
  }, [load])

  const resetDraft = () => {
    setDraft(emptyDraft)
    setEditingFilterId(null)
    setEditLoading(false)
    setEditLoadSucceeded(false)
    setCurrentLoadId(null)
    currentLoadIdRef.current = null
  }

  const updateDraft = (updates: Partial<FilterDraft>) => {
    setDraft(prev => ({ ...prev, ...updates }))
  }

  const validateFilename = (filename: string, currentFilename?: string) => {
    const normalized = filename.trim()
    const normalizedLower = normalized.toLowerCase()
    const currentFilenameLower = currentFilename?.trim().toLowerCase()
    if (!normalized) {
      toast.error('Укажите имя файла фильтра')
      return false
    }

    const existingFilters = useConfigStore.getState().config?.filters || []
    const hasCollision = existingFilters.some(filter =>
      filter.filename.trim().toLowerCase() === normalizedLower
      && filter.filename.trim().toLowerCase() !== currentFilenameLower,
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

  const persistFilters = async (nextFilters: FilterType[], previousFilters: FilterType[]) => {
    const mutationId = ++latestMutationIdRef.current
    setFilters(nextFilters)
    try {
      await useConfigStore.getState().save()
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
    const updatedFilters = currentFilters.map(filter =>
      filter.id === filterId ? { ...filter, active: !filter.active } : filter,
    )
    void persistFilters(updatedFilters, currentFilters)
      .then(() => restartIfConnected()
        .then(() => {
          notifyConfigApplied('Фильтр обновлён')
        })
        .catch((e) => {
          toast.error(`Ошибка применения фильтров: ${e instanceof Error ? e.message : String(e)}`)
        }))
      .catch((e) => {
        toast.error(`Ошибка сохранения фильтров: ${e instanceof Error ? e.message : String(e)}`)
      })
  }

  const handleCreateFilter = async () => {
    if (createInFlight)
      return
    if (!draft.name.trim() || !draft.filename.trim())
      return

    const nextFilename = draft.filename.trim()
    if (!validateFilename(nextFilename))
      return

    setCreateInFlight(true)
    try {
      const newFilter: FilterType = {
        id: `filter-${crypto.randomUUID()}`,
        name: draft.name.trim(),
        filename: nextFilename,
        active: true,
      }

      await tauri.saveFilterFile(nextFilename, draft.content ?? '')

      const currentFilters = useConfigStore.getState().config?.filters || []
      await persistFilters([...currentFilters, newFilter], currentFilters)
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
      filename: filter.filename,
      content: '',
    })

    try {
      const content = await tauri.loadFilterFile(filter.filename)
      if (currentLoadIdRef.current !== loadId) {
        return
      }

      setDraft({
        name: filter.name,
        filename: filter.filename,
        content,
      })
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

    const nextFilename = draft.filename.trim()
    if (!validateFilename(nextFilename, targetFilter.filename))
      return

    const renamed = targetFilter.filename.trim().toLowerCase() !== nextFilename.trim().toLowerCase()
    setEditInFlight(true)
    const originalContent = await tauri.loadFilterFile(targetFilter.filename).catch(() => draft.content)
    try {
      await tauri.saveFilterFile(nextFilename, draft.content)
      if (renamed) {
        try {
          await tauri.deleteFilterFile(targetFilter.filename)
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
            }
          : filter,
      )

      await persistFilters(updatedFilters, currentFilters)
      resetDraft()
      setEditDialogOpen(false)
      toast.success('Фильтр сохранён')
    }
    catch (e) {
      if (renamed) {
        await tauri.deleteFilterFile(nextFilename).catch(() => {})
      }
      await tauri.saveFilterFile(targetFilter.filename, originalContent).catch(() => {})
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
      try {
        originalContent = await tauri.loadFilterFile(filter.filename)
      }
      catch {}
      await tauri.deleteFilterFile(filter.filename)
      const currentFilters = useConfigStore.getState().config?.filters || []
      const nextFilters = currentFilters.filter(item => item.id !== filter.id)
      try {
        await persistFilters(nextFilters, currentFilters)
      }
      catch (e) {
        if (originalContent !== undefined) {
          await tauri.saveFilterFile(filter.filename, originalContent).catch(() => {})
        }
        throw e
      }
      if (editingFilterId === filter.id) {
        resetDraft()
        setEditDialogOpen(false)
      }
      toast.success('Фильтр удалён')
    }
    catch (e) {
      toast.error(`Ошибка удаления фильтра: ${e instanceof Error ? e.message : String(e)}`)
    }
    finally {
      setDeleteInFlightId(null)
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
    <ScrollArea className="h-full">
      <div className="space-y-6 p-6">
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-2xl font-medium">Фильтры</h1>
            <p className="mt-1 text-sm text-muted-foreground">
              WinDivert фильтры для отсечения полезной нагрузки
            </p>
          </div>
          <div className="flex items-center gap-2">
            <Button onClick={() => setCreateDialogOpen(true)}>
              <Plus className="mr-2 h-4 w-4" />
              Новый фильтр
            </Button>
            <Button
              variant="outline"
              size="icon"
              title="Открыть папку ~/.zapret/filters"
              aria-label="Открыть папку ~/.zapret/filters"
              onClick={async () => {
                try {
                  const filtersPath = await tauri.getFiltersPath()
                  await openPath(filtersPath)
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
          {config.filters?.map((filter: FilterType) => (
            <div
              key={filter.id}
              className="flex min-h-20 items-center justify-between rounded-lg border bg-card p-4"
            >
              <div className="flex min-w-0 items-center gap-3">
                <Filter className="h-4 w-4 text-muted-foreground" />
                <div className="min-w-0 space-y-1">
                  <Label htmlFor={filter.id} className="block cursor-pointer truncate text-sm font-normal">
                    {filter.name}
                  </Label>
                  <p className="truncate text-xs text-muted-foreground">
                    {filter.filename}
                  </p>
                </div>
              </div>
              <div className="flex items-center gap-2">
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
                      variant="ghost"
                      size="icon"
                      className="bg-red-500/10 text-red-600 hover:bg-red-500/20 hover:text-red-700 dark:text-red-400 dark:hover:text-red-300"
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
          ))}
        </div>

        <Dialog
          open={createDialogOpen}
          onOpenChange={(open) => {
            setCreateDialogOpen(open)
            if (!open)
              resetDraft()
          }}
        >
          <DialogContent className="max-w-2xl">
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
              </div>
              <div className="space-y-2">
                <Label htmlFor="filter-content">Содержимое фильтра</Label>
                <Textarea
                  id="filter-content"
                  value={draft.content}
                  onChange={e => updateDraft({ content: e.target.value })}
                  placeholder="WinDivert фильтр..."
                  rows={10}
                  className="font-mono text-sm"
                />
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
          <DialogContent className="max-w-3xl">
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
                <Textarea
                  id="edit-filter-content"
                  value={draft.content}
                  onChange={e => updateDraft({ content: e.target.value })}
                  placeholder="WinDivert фильтр..."
                  rows={16}
                  className="font-mono text-sm"
                  disabled={editLoading}
                />
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
    </ScrollArea>
  )
}
