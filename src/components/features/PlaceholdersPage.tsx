import type { Placeholder } from '@/lib/types'
import { FolderOpen, Loader2, Pencil, Plus, Trash2 } from 'lucide-react'
import { useEffect, useState } from 'react'
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

  const { config, loading, load, save, addPlaceholder, updatePlaceholder, deletePlaceholder }
    = useConfigStore()

  useEffect(() => {
    load().catch(console.error)
  }, [])

  useEffect(() => {
    if (config) {
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

  const handleDelete = (index: number) => {
    deletePlaceholder(index)
    toast.success('Плейсхолдер удалён')
  }

  if (loading) {
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
          <h1 className="text-2xl font-semibold">Плейсхолдеры</h1>
          <p className="text-sm text-muted-foreground mt-1">
            Замена плейсхолдеров на пути к бинарным файлам
          </p>
        </div>
        <Button variant="outline" size="icon" onClick={() => tauri.openZapretDirectory()} title="Открыть папку ~/.zapret">
          <FolderOpen className="w-4 h-4" />
        </Button>
      </div>

      <Card>
        <CardHeader className="flex flex-row items-center justify-between">
          <CardTitle className="text-lg">Таблица плейсхолдеров</CardTitle>
          <Button size="sm" onClick={() => setAddOpen(true)}>
            <Plus className="w-4 h-4 mr-2" />
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
                      <TableRow key={placeholder.name}>
                        <TableCell className="font-mono whitespace-nowrap">
                          {'{{'}
                          {placeholder.name}
                          {'}}'}
                        </TableCell>
                        <TableCell className="font-mono text-muted-foreground max-w-0 w-full">
                          <div className="relative overflow-hidden whitespace-nowrap" title={placeholder.path}>
                            {placeholder.path}
                            <div className="absolute inset-y-0 right-0 w-16 bg-gradient-to-l from-card to-transparent pointer-events-none" />
                          </div>
                        </TableCell>
                        <TableCell>
                          <div className="flex justify-end gap-1">
                            <Button
                              variant="ghost"
                              size="sm"
                              onClick={() => handleEdit(index, placeholder)}
                            >
                              <Pencil className="w-4 h-4" />
                            </Button>
                            <Button
                              variant="ghost"
                              size="sm"
                              onClick={() => handleDelete(index)}
                            >
                              <Trash2 className="w-4 h-4" />
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
          <div className="py-4 space-y-4">
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
          <div className="py-4 space-y-4">
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
