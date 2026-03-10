import type { Placeholder } from '@/lib/types'
import { FileCode, FolderOpen, Loader2, Pencil, Plus, Trash2 } from 'lucide-react'
import { useEffect, useRef, useState } from 'react'
import { toast } from 'sonner'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { ScrollArea } from '@/components/ui/scroll-area'
import * as tauri from '@/lib/tauri'
import { useConfigStore } from '@/stores/config.store'

export function PlaceholdersPage() {
  const [editingIndex, setEditingIndex] = useState<number | null>(null)
  const [editName, setEditName] = useState('')
  const [editPath, setEditPath] = useState('')
  const [addOpen, setAddOpen] = useState(false)
  const [newName, setNewName] = useState('')
  const [newPath, setNewPath] = useState('')
  const isInitialLoadRef = useRef(true)
  const isSavingRef = useRef(false)

  const { config, loading, load, save, addPlaceholder, updatePlaceholder, deletePlaceholder }
    = useConfigStore()

  useEffect(() => {
    load().then(() => {
      isInitialLoadRef.current = false
    }).catch(console.error)
  }, [])

  useEffect(() => {
    if (config && !isInitialLoadRef.current && !isSavingRef.current) {
      save().catch(console.error)
    }
  }, [config])

  const handleAdd = () => {
    if (newName.trim() && newPath.trim()) {
      addPlaceholder(newName.trim(), newPath.trim())
      setNewName('')
      setNewPath('')
      setAddOpen(false)
    }
  }

  const handleEdit = (index: number, placeholder: Placeholder) => {
    setEditingIndex(index)
    setEditName(placeholder.name)
    setEditPath(placeholder.path)
  }

  const handleSaveEdit = () => {
    if (editingIndex !== null) {
      const trimmedName = editName.trim()
      const trimmedPath = editPath.trim()
      if (!trimmedName || !trimmedPath) {
        return
      }
      updatePlaceholder(editingIndex, trimmedName, trimmedPath)
      setEditingIndex(null)
    }
  }

  const handleDelete = async (index: number) => {
    if (isSavingRef.current)
      return
    const placeholderToDelete = config?.placeholders[index]
    deletePlaceholder(index)
    isSavingRef.current = true
    try {
      await save()
      toast.success('Плейсхолдер удалён')
    }
    catch (e) {
      toast.error(`Ошибка сохранения: ${e}`)
      if (placeholderToDelete) {
        addPlaceholder(placeholderToDelete.name, placeholderToDelete.path)
      }
    }
    finally {
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
    <ScrollArea className="h-full">
      <div className="space-y-6 p-6">
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-2xl font-medium">Плейсхолдеры</h1>
            <p className="mt-1 text-sm text-muted-foreground">
              Замена плейсхолдеров на пути к бинарным файлам
            </p>
          </div>
          <div className="flex items-center gap-1">
            <Button onClick={() => setAddOpen(true)}>
              <Plus className="mr-2 h-4 w-4" />
              Новый плейсхолдер
            </Button>
            <Button
              variant="outline"
              size="icon"
              onClick={() => tauri.openZapretDirectory()}
              title="Открыть папку ~/.zapret"
              aria-label="Открыть папку ~/.zapret"
            >
              <FolderOpen className="h-4 w-4" />
            </Button>
          </div>
        </div>

        <div className="space-y-3">
          {!config || config.placeholders.length === 0
            ? (
                <div className="text-muted-foreground flex min-h-32 items-center justify-center rounded-lg border border-dashed">
                  Нет плейсхолдеров
                </div>
              )
            : (
                config.placeholders.map((placeholder: Placeholder, index: number) => (
                  <div
                    key={`${index}-${placeholder.name}`}
                    className="bg-card flex min-h-20 items-center justify-between gap-4 rounded-lg border p-4"
                  >
                    <div className="flex min-w-0 items-center gap-3">
                      <FileCode className="h-4 w-4 text-muted-foreground" />
                      <div className="min-w-0 flex-1 space-y-1">
                        <div className="truncate text-sm font-normal text-foreground">
                          {'{{'}
                          {placeholder.name}
                          {'}}'}
                        </div>
                        <div className="truncate text-xs text-muted-foreground" title={placeholder.path}>
                          {placeholder.path}
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
                        className="border-red-500/30 bg-red-500/10 text-red-700 hover:bg-red-500/20 hover:text-red-800 dark:text-red-400 dark:hover:text-red-300"
                        aria-label={`Удалить плейсхолдер ${placeholder.name}`}
                        onClick={() => handleDelete(index)}
                      >
                        <Trash2 className="h-4 w-4" />
                      </Button>
                    </div>
                  </div>
                ))
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
                placeholder="Название (например TLS_CLIENTHELLO_GOOGLE)"
                value={newName}
                onChange={e => setNewName(e.target.value)}
              />
              <Input
                aria-label="Путь плейсхолдера"
                placeholder="Путь к файлу (например ~/.zapret/tls.bin)"
                value={newPath}
                onChange={e => setNewPath(e.target.value)}
              />
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
              <Input
                aria-label="Название плейсхолдера"
                placeholder="Название"
                value={editName}
                onChange={e => setEditName(e.target.value)}
              />
              <Input
                aria-label="Путь плейсхолдера"
                placeholder="Путь к файлу"
                value={editPath}
                onChange={e => setEditPath(e.target.value)}
              />
            </div>
            <DialogFooter>
              <Button variant="outline" onClick={() => setEditingIndex(null)}>
                Отмена
              </Button>
              <Button onClick={handleSaveEdit}>Сохранить</Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      </div>
    </ScrollArea>
  )
}
