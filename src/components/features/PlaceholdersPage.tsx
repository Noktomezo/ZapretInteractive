import type { Placeholder } from '@/lib/types'
import { FolderOpen, Loader2, Pencil, Plus, Trash2 } from 'lucide-react'
import { useEffect, useRef, useState } from 'react'
import { toast } from 'sonner'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
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
    <div className="space-y-6 p-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-semibold">Плейсхолдеры</h1>
          <p className="mt-1 text-sm text-muted-foreground">
            Замена плейсхолдеров на пути к бинарным файлам
          </p>
        </div>
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

      <Card>
        <CardHeader className="flex flex-row items-center justify-between">
          <CardTitle className="text-lg">Таблица плейсхолдеров</CardTitle>
          <Button size="sm" onClick={() => setAddOpen(true)}>
            <Plus className="mr-2 h-4 w-4" />
            Добавить
          </Button>
        </CardHeader>
        <CardContent>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead className="w-40">Название</TableHead>
                <TableHead>Путь</TableHead>
                <TableHead className="w-24">Действия</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {config?.placeholders.length === 0
                ? (
                    <TableRow>
                      <TableCell colSpan={3} className="text-center text-muted-foreground">
                        Нет плейсхолдеров
                      </TableCell>
                    </TableRow>
                  )
                : (
                    config?.placeholders.map((placeholder: Placeholder, index: number) => (
                      <TableRow key={`${index}-${placeholder.name}`}>
                        <TableCell className="font-mono whitespace-nowrap">
                          {'{{'}
                          {placeholder.name}
                          {'}}'}
                        </TableCell>
                        <TableCell className="max-w-0 w-full font-mono text-muted-foreground">
                          <div className="relative overflow-hidden whitespace-nowrap" title={placeholder.path}>
                            {placeholder.path}
                            <div className="pointer-events-none absolute inset-y-0 right-0 w-16 bg-gradient-to-l from-card to-transparent" />
                          </div>
                        </TableCell>
                        <TableCell>
                          <div className="flex justify-end gap-1">
                            <Button
                              variant="ghost"
                              size="sm"
                              className="bg-yellow-500/10 text-yellow-600 hover:bg-yellow-500/20 hover:text-yellow-700 dark:text-yellow-400 dark:hover:text-yellow-300"
                              aria-label={`Редактировать плейсхолдер ${placeholder.name}`}
                              onClick={() => handleEdit(index, placeholder)}
                            >
                              <Pencil className="h-4 w-4" />
                            </Button>
                            <Button
                              variant="ghost"
                              size="sm"
                              className="bg-red-500/10 text-red-600 hover:bg-red-500/20 hover:text-red-700 dark:text-red-400 dark:hover:text-red-300"
                              aria-label={`Удалить плейсхолдер ${placeholder.name}`}
                              onClick={() => handleDelete(index)}
                            >
                              <Trash2 className="h-4 w-4" />
                            </Button>
                          </div>
                        </TableCell>
                      </TableRow>
                    ))
                  )}
            </TableBody>
          </Table>
        </CardContent>
      </Card>

      <Dialog open={addOpen} onOpenChange={setAddOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Новый плейсхолдер</DialogTitle>
          </DialogHeader>
          <div className="space-y-4 py-4">
            <Input
              placeholder="Название (например TLS_CLIENTHELLO_GOOGLE)"
              value={newName}
              onChange={e => setNewName(e.target.value)}
            />
            <Input
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
              placeholder="Название"
              value={editName}
              onChange={e => setEditName(e.target.value)}
            />
            <Input
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
  )
}
