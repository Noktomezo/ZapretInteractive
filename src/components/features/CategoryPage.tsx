import type { Strategy } from '@/lib/types'
import { Link, useParams } from '@tanstack/react-router'
import { ArrowLeft, Loader2, MoreHorizontal, Pencil, Plus, Trash2 } from 'lucide-react'
import { useEffect, useState } from 'react'
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
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { Input } from '@/components/ui/input'
import { Textarea } from '@/components/ui/textarea'
import { useConfigStore } from '@/stores/config.store'

export function CategoryPage() {
  const { categoryId } = useParams({ from: '/strategies/$categoryId' })

  const [editingStrategy, setEditingStrategy] = useState<Strategy | null>(null)
  const [newStrategyOpen, setNewStrategyOpen] = useState(false)
  const [newStrategyName, setNewStrategyName] = useState('')
  const [newStrategyContent, setNewStrategyContent] = useState('')
  const [editingName, setEditingName] = useState('')
  const [editingContent, setEditingContent] = useState('')
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false)
  const [renameDialogOpen, setRenameDialogOpen] = useState(false)
  const [newCategoryName, setNewCategoryName] = useState('')

  const { config, loading, load, save, updateCategory, deleteCategory, addStrategy, updateStrategy, deleteStrategy, setActiveStrategy, clearActiveStrategy, clearAllActiveStrategies } = useConfigStore()

  useEffect(() => {
    load()
  }, [])

  useEffect(() => {
    if (config) {
      save()
    }
  }, [config])

  const category = config?.categories.find(c => c.id === categoryId)

  const handleAddStrategy = () => {
    if (newStrategyName.trim() && newStrategyContent.trim() && categoryId) {
      addStrategy(categoryId, newStrategyName.trim(), newStrategyContent.trim())
      setNewStrategyName('')
      setNewStrategyContent('')
      setNewStrategyOpen(false)
    }
  }

  const handleEditStrategy = (strategy: Strategy) => {
    setEditingStrategy(strategy)
    setEditingName(strategy.name)
    setEditingContent(strategy.content)
  }

  const handleSaveEdit = () => {
    if (editingStrategy && categoryId) {
      updateStrategy(categoryId, editingStrategy.id, {
        name: editingName,
        content: editingContent,
      })
      setEditingStrategy(null)
    }
  }

  const handleSetActive = (strategyId: string) => {
    if (categoryId) {
      setActiveStrategy(categoryId, strategyId)
    }
  }

  const handleClearActive = (strategyId: string) => {
    if (categoryId) {
      clearActiveStrategy(categoryId, strategyId)
    }
  }

  const handleClearAllActive = () => {
    if (categoryId) {
      clearAllActiveStrategies(categoryId)
    }
  }

  const handleDeleteStrategy = (strategyId: string) => {
    if (categoryId) {
      deleteStrategy(categoryId, strategyId)
      toast.success('Стратегия удалена')
    }
  }

  const handleDeleteCategory = () => {
    if (categoryId) {
      deleteCategory(categoryId)
      setDeleteDialogOpen(false)
      toast.success('Категория удалена')
    }
  }

  const handleRenameCategory = () => {
    if (categoryId && newCategoryName.trim()) {
      updateCategory(categoryId, newCategoryName.trim())
      setRenameDialogOpen(false)
    }
  }

  const openRenameDialog = () => {
    if (category) {
      setNewCategoryName(category.name)
      setRenameDialogOpen(true)
    }
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <Loader2 className="w-6 h-6 animate-spin" />
      </div>
    )
  }

  if (!category) {
    return (
      <div className="p-6 space-y-6">
        <Link to="/strategies" className="flex items-center gap-2 text-muted-foreground hover:text-foreground">
          <ArrowLeft className="w-4 h-4" />
          Назад к категориям
        </Link>
        <p className="text-muted-foreground">Категория не найдена</p>
      </div>
    )
  }

  return (
    <div className="p-6 space-y-6">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-4">
          <Link to="/strategies" className="text-muted-foreground hover:text-foreground cursor-pointer">
            <ArrowLeft className="w-5 h-5" />
          </Link>
          <div>
            <h1 className="text-2xl font-semibold">{category.name}</h1>
            <p className="text-sm text-muted-foreground mt-1">
              {category.strategies.length}
              {' '}
              стратеги
              {category.strategies.length === 1 ? 'я' : category.strategies.length < 5 ? 'и' : 'й'}
            </p>
          </div>
        </div>
        <div className="flex gap-2">
          <Button variant="outline" size="sm" onClick={openRenameDialog}>
            <Pencil className="w-4 h-4 mr-1" />
            Переименовать
          </Button>
          <AlertDialog open={deleteDialogOpen} onOpenChange={setDeleteDialogOpen}>
            <AlertDialogTrigger asChild>
              <Button variant="ghost" size="sm">
                <Trash2 className="w-4 h-4" />
              </Button>
            </AlertDialogTrigger>
            <AlertDialogContent>
              <AlertDialogHeader>
                <AlertDialogTitle>Удалить категорию?</AlertDialogTitle>
                <AlertDialogDescription>
                  Категория «
                  {category.name}
                  » и все её стратегии будут удалены. Это действие нельзя отменить.
                </AlertDialogDescription>
              </AlertDialogHeader>
              <AlertDialogFooter>
                <AlertDialogCancel>Отмена</AlertDialogCancel>
                <AlertDialogAction onClick={handleDeleteCategory} className="bg-destructive text-destructive-foreground hover:bg-destructive/90">
                  Удалить
                </AlertDialogAction>
              </AlertDialogFooter>
            </AlertDialogContent>
          </AlertDialog>
        </div>
      </div>

      <div className="flex gap-2">
        <Button onClick={() => setNewStrategyOpen(true)}>
          <Plus className="w-4 h-4 mr-2" />
          Новая стратегия
        </Button>
        {category.strategies.some(s => s.active) && (
          <Button variant="outline" onClick={handleClearAllActive}>
            Деактивировать текущую
          </Button>
        )}
      </div>

      <div className="space-y-4">
        {category.strategies.length === 0
          ? (
              <p className="text-sm text-muted-foreground">Нет стратегий</p>
            )
          : (
              category.strategies.map((strategy: Strategy) => (
                <div
                  key={strategy.id}
                  className="border border-border rounded-lg p-4 space-y-3"
                >
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-3">
                      <span className="font-medium">{strategy.name}</span>
                      {strategy.active && (
                        <span className="text-xs bg-green-600 text-white px-2 py-0.5 rounded">
                          активна
                        </span>
                      )}
                    </div>
                    <div className="flex items-center gap-2">
                      {strategy.active
                        ? (
                            <Button
                              variant="outline"
                              size="sm"
                              onClick={() => handleClearActive(strategy.id)}
                            >
                              Деактивировать
                            </Button>
                          )
                        : (
                            <Button
                              variant="outline"
                              size="sm"
                              onClick={() => handleSetActive(strategy.id)}
                            >
                              Активировать
                            </Button>
                          )}
                      <DropdownMenu>
                        <DropdownMenuTrigger asChild>
                          <Button variant="ghost" size="sm">
                            <MoreHorizontal className="w-4 h-4" />
                          </Button>
                        </DropdownMenuTrigger>
                        <DropdownMenuContent>
                          <DropdownMenuItem onClick={() => handleEditStrategy(strategy)}>
                            <Pencil className="w-4 h-4 mr-2" />
                            Редактировать
                          </DropdownMenuItem>
                          <DropdownMenuItem
                            onClick={() => handleDeleteStrategy(strategy.id)}
                            className="text-destructive"
                          >
                            <Trash2 className="w-4 h-4 mr-2" />
                            Удалить
                          </DropdownMenuItem>
                        </DropdownMenuContent>
                      </DropdownMenu>
                    </div>
                  </div>
                  <pre className="text-xs bg-muted p-3 rounded-md overflow-x-auto">
                    {strategy.content}
                  </pre>
                </div>
              ))
            )}
      </div>

      <Dialog open={newStrategyOpen} onOpenChange={setNewStrategyOpen}>
        <DialogContent className="max-w-2xl">
          <DialogHeader>
            <DialogTitle>Новая стратегия</DialogTitle>
          </DialogHeader>
          <div className="py-4 space-y-4">
            <Input
              placeholder="Название стратегии"
              value={newStrategyName}
              onChange={e => setNewStrategyName(e.target.value)}
            />
            <Textarea
              placeholder="--dpi-desync=fake&#10;--dpi-desync-autottl=2"
              value={newStrategyContent}
              onChange={e => setNewStrategyContent(e.target.value)}
              rows={8}
              className="font-mono text-sm"
            />
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setNewStrategyOpen(false)}>
              Отмена
            </Button>
            <Button onClick={handleAddStrategy}>Создать</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog open={!!editingStrategy} onOpenChange={() => setEditingStrategy(null)}>
        <DialogContent className="max-w-2xl">
          <DialogHeader>
            <DialogTitle>Редактировать стратегию</DialogTitle>
          </DialogHeader>
          <div className="py-4 space-y-4">
            <Input
              placeholder="Название стратегии"
              value={editingName}
              onChange={e => setEditingName(e.target.value)}
            />
            <Textarea
              placeholder="--dpi-desync=fake&#10;--dpi-desync-autottl=2"
              value={editingContent}
              onChange={e => setEditingContent(e.target.value)}
              rows={8}
              className="font-mono text-sm"
            />
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setEditingStrategy(null)}>
              Отмена
            </Button>
            <Button onClick={handleSaveEdit}>Сохранить</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog open={renameDialogOpen} onOpenChange={setRenameDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Переименовать категорию</DialogTitle>
          </DialogHeader>
          <div className="py-4">
            <Input
              placeholder="Название категории"
              value={newCategoryName}
              onChange={e => setNewCategoryName(e.target.value)}
              onKeyDown={e => e.key === 'Enter' && handleRenameCategory()}
            />
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setRenameDialogOpen(false)}>
              Отмена
            </Button>
            <Button onClick={handleRenameCategory}>Сохранить</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  )
}
