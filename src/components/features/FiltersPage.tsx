import type { Filter as FilterType } from '@/lib/types'
import { Filter, Loader2, Pencil, Plus, Trash2 } from 'lucide-react'
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
import { Switch } from '@/components/ui/switch'
import { Textarea } from '@/components/ui/textarea'
import * as tauri from '@/lib/tauri'
import { useConfigStore } from '@/stores/config.store'

type FilterDraft = {
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
  const { config, loading, load, save, setFilters } = useConfigStore()
  const [createDialogOpen, setCreateDialogOpen] = useState(false)
  const [editDialogOpen, setEditDialogOpen] = useState(false)
  const [editingFilterId, setEditingFilterId] = useState<string | null>(null)
  const [draft, setDraft] = useState<FilterDraft>(emptyDraft)
  const [editLoading, setEditLoading] = useState(false)
  const isInitialLoadRef = useRef(true)

  useEffect(() => {
    load().then(() => {
      isInitialLoadRef.current = false
    })
  }, [])

  useEffect(() => {
    if (config && !isInitialLoadRef.current) {
      save().catch((e) => {
        toast.error(`Ошибка сохранения фильтров: ${e instanceof Error ? e.message : String(e)}`)
      })
    }
  }, [config])

  const resetDraft = () => {
    setDraft(emptyDraft)
    setEditingFilterId(null)
    setEditLoading(false)
  }

  const updateDraft = (updates: Partial<FilterDraft>) => {
    setDraft(prev => ({ ...prev, ...updates }))
  }

  const handleToggleFilter = (filterId: string) => {
    const currentFilters = useConfigStore.getState().config?.filters || []
    const updatedFilters = currentFilters.map(filter =>
      filter.id === filterId ? { ...filter, active: !filter.active } : filter,
    )
    setFilters(updatedFilters)
  }

  const handleCreateFilter = async () => {
    if (!draft.name.trim() || !draft.filename.trim())
      return

    try {
      const newFilter: FilterType = {
        id: `filter-${crypto.randomUUID()}`,
        name: draft.name.trim(),
        filename: draft.filename.trim(),
        active: true,
      }

      if (draft.content.trim()) {
        await tauri.saveFilterFile(draft.filename.trim(), draft.content)
      }

      const currentFilters = useConfigStore.getState().config?.filters || []
      setFilters([...currentFilters, newFilter])
      resetDraft()
      setCreateDialogOpen(false)
      toast.success('Фильтр создан')
    }
    catch (e) {
      toast.error(`Ошибка создания фильтра: ${e instanceof Error ? e.message : String(e)}`)
    }
  }

  const openEditDialog = async (filter: FilterType) => {
    setEditingFilterId(filter.id)
    setEditDialogOpen(true)
    setEditLoading(true)
    setDraft({
      name: filter.name,
      filename: filter.filename,
      content: '',
    })

    try {
      const content = await tauri.loadFilterFile(filter.filename)
      setDraft({
        name: filter.name,
        filename: filter.filename,
        content,
      })
    }
    catch (e) {
      toast.error(`Ошибка загрузки фильтра: ${e instanceof Error ? e.message : String(e)}`)
    }
    finally {
      setEditLoading(false)
    }
  }

  const handleSaveEdit = async () => {
    if (!editingFilterId || !draft.name.trim() || !draft.filename.trim())
      return

    const currentFilters = useConfigStore.getState().config?.filters || []
    const targetFilter = currentFilters.find(filter => filter.id === editingFilterId)
    if (!targetFilter)
      return

    try {
      await tauri.saveFilterFile(draft.filename.trim(), draft.content)
      if (targetFilter.filename !== draft.filename.trim()) {
        await tauri.deleteFilterFile(targetFilter.filename)
      }

      const updatedFilters = currentFilters.map(filter =>
        filter.id === editingFilterId
          ? {
              ...filter,
              name: draft.name.trim(),
              filename: draft.filename.trim(),
            }
          : filter,
      )

      setFilters(updatedFilters)
      resetDraft()
      setEditDialogOpen(false)
      toast.success('Фильтр сохранён')
    }
    catch (e) {
      toast.error(`Ошибка сохранения фильтра: ${e instanceof Error ? e.message : String(e)}`)
    }
  }

  const handleDeleteFilter = async (filter: FilterType) => {
    try {
      await tauri.deleteFilterFile(filter.filename)
      const currentFilters = useConfigStore.getState().config?.filters || []
      setFilters(currentFilters.filter(item => item.id !== filter.id))
      if (editingFilterId === filter.id) {
        resetDraft()
        setEditDialogOpen(false)
      }
      toast.success('Фильтр удалён')
    }
    catch (e) {
      toast.error(`Ошибка удаления фильтра: ${e instanceof Error ? e.message : String(e)}`)
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
    <div className="space-y-6 p-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-semibold">Фильтры</h1>
          <p className="mt-1 text-sm text-muted-foreground">
            WinDivert фильтры для отсечения полезной нагрузки
          </p>
        </div>
        <Button onClick={() => setCreateDialogOpen(true)}>
          <Plus className="mr-2 h-4 w-4" />
          Новый фильтр
        </Button>
      </div>

      <div className="space-y-3">
        {config.filters?.map((filter: FilterType) => (
          <div
            key={filter.id}
            className="flex items-center justify-between rounded-lg border bg-card p-4"
          >
            <div className="flex items-center gap-3">
              <Filter className="h-4 w-4 text-muted-foreground" />
              <div>
                <Label htmlFor={filter.id} className="cursor-pointer font-medium">
                  {filter.name}
                </Label>
                <p className="font-mono text-xs text-muted-foreground">
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
                variant="ghost"
                size="icon"
                className="h-8 w-8 text-muted-foreground"
                onClick={() => openEditDialog(filter)}
              >
                <Pencil className="h-4 w-4" />
              </Button>
              <AlertDialog>
                <AlertDialogTrigger asChild>
                  <Button variant="ghost" size="icon" className="h-8 w-8 text-muted-foreground hover:text-destructive">
                    <Trash2 className="h-4 w-4" />
                  </Button>
                </AlertDialogTrigger>
                <AlertDialogContent>
                  <AlertDialogHeader>
                    <AlertDialogTitle>Удалить фильтр?</AlertDialogTitle>
                    <AlertDialogDescription>
                      Фильтр "{filter.name}" будет удалён из списка, а файл {filter.filename} будет удалён с диска.
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
                placeholder="windivert_part.discord_media.txt"
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
            <Button onClick={handleCreateFilter} disabled={!draft.name.trim() || !draft.filename.trim()}>
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
                  placeholder="windivert_part.discord_media.txt"
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
              {editLoading && (
                <p className="text-xs text-muted-foreground">Загружаю содержимое файла...</p>
              )}
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setEditDialogOpen(false)}>
              Закрыть
            </Button>
            <Button onClick={handleSaveEdit} disabled={editLoading || !draft.name.trim() || !draft.filename.trim()}>
              Сохранить
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  )
}


