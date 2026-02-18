import type { Filter as FilterType } from '@/lib/types'
import { Filter, Loader2, Plus, Trash2 } from 'lucide-react'
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

export function FiltersPage() {
  const { config, loading, load, save, setFilters } = useConfigStore()
  const [dialogOpen, setDialogOpen] = useState(false)
  const [newName, setNewName] = useState('')
  const [newFilename, setNewFilename] = useState('')
  const [newContent, setNewContent] = useState('')
  const isInitialLoadRef = useRef(true)

  useEffect(() => {
    load().then(() => {
      isInitialLoadRef.current = false
    })
  }, [])

  useEffect(() => {
    if (config && !isInitialLoadRef.current) {
      save()
    }
  }, [config])

  const handleToggleFilter = (filterId: string) => {
    if (!config?.filters)
      return

    const updatedFilters = config.filters.map(f =>
      f.id === filterId ? { ...f, active: !f.active } : f,
    )
    setFilters(updatedFilters)
  }

  const handleCreateFilter = async () => {
    if (!newName.trim() || !newFilename.trim())
      return

    try {
      const newFilter: FilterType = {
        id: `filter-${crypto.randomUUID()}`,
        name: newName.trim(),
        filename: newFilename.trim(),
        active: true,
      }

      if (newContent.trim()) {
        await tauri.saveFilterFile(newFilename.trim(), newContent.trim())
      }

      const currentFilters = useConfigStore.getState().config?.filters || []
      setFilters([...currentFilters, newFilter])
      setNewName('')
      setNewFilename('')
      setNewContent('')
      setDialogOpen(false)
      toast.success('Фильтр создан')
    }
    catch (e) {
      toast.error(`Ошибка создания фильтра: ${e}`)
    }
  }

  const handleDeleteFilter = async (filter: FilterType) => {
    try {
      await tauri.deleteFilterFile(filter.filename)
      const currentFilters = useConfigStore.getState().config?.filters || []
      setFilters(currentFilters.filter(f => f.id !== filter.id))
      toast.success('Фильтр удалён')
    }
    catch (e) {
      toast.error(`Ошибка удаления фильтра: ${e}`)
    }
  }

  if (loading || !config) {
    return (
      <div className="flex items-center justify-center h-full">
        <Loader2 className="w-6 h-6 animate-spin" />
      </div>
    )
  }

  return (
    <div className="p-6 space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-semibold">Фильтры</h1>
          <p className="text-sm text-muted-foreground mt-1">
            WinDivert фильтры для отсечения полезной нагрузки
          </p>
        </div>
        <Button onClick={() => setDialogOpen(true)}>
          <Plus className="w-4 h-4 mr-2" />
          Новый фильтр
        </Button>
      </div>

      <div className="space-y-3">
        {config.filters?.map((filter: FilterType) => (
          <div
            key={filter.id}
            className="flex items-center justify-between p-4 rounded-lg border bg-card"
          >
            <div className="flex items-center gap-3">
              <Filter className="w-4 h-4 text-muted-foreground" />
              <div>
                <Label
                  htmlFor={filter.id}
                  className="font-medium cursor-pointer"
                >
                  {filter.name}
                </Label>
                <p className="text-xs text-muted-foreground font-mono">
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
              <AlertDialog>
                <AlertDialogTrigger asChild>
                  <Button variant="ghost" size="icon" className="h-8 w-8 text-muted-foreground hover:text-destructive">
                    <Trash2 className="w-4 h-4" />
                  </Button>
                </AlertDialogTrigger>
                <AlertDialogContent>
                  <AlertDialogHeader>
                    <AlertDialogTitle>Удалить фильтр?</AlertDialogTitle>
                    <AlertDialogDescription>
                      Фильтр "
                      {filter.name}
                      " будет удалён из списка и файл
                      {' '}
                      {filter.filename}
                      {' '}
                      будет удалён.
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

      <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Новый фильтр</DialogTitle>
          </DialogHeader>
          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <Label htmlFor="filter-name">Название</Label>
              <Input
                id="filter-name"
                value={newName}
                onChange={e => setNewName(e.target.value)}
                placeholder="Discord Media"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="filter-filename">Имя файла</Label>
              <Input
                id="filter-filename"
                value={newFilename}
                onChange={e => setNewFilename(e.target.value)}
                placeholder="windivert_part.discord_media.txt"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="filter-content">Содержимое фильтра</Label>
              <Textarea
                id="filter-content"
                value={newContent}
                onChange={e => setNewContent(e.target.value)}
                placeholder="WinDivert фильтр..."
                rows={8}
                className="font-mono text-sm"
              />
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDialogOpen(false)}>
              Отмена
            </Button>
            <Button onClick={handleCreateFilter} disabled={!newName.trim() || !newFilename.trim()}>
              Создать
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  )
}
