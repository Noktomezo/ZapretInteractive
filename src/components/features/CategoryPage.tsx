import type { Strategy } from '@/lib/types'
import { Link, useNavigate, useParams } from '@tanstack/react-router'
import { ArrowLeft, BrushCleaning, Check, Loader2, Pencil, Plus, Trash2 } from 'lucide-react'
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
import { ScrollArea } from '@/components/ui/scroll-area'
import { Textarea } from '@/components/ui/textarea'
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip'
import { useConfigStore } from '@/stores/config.store'
import { useConnectionStore } from '@/stores/connection.store'

const DEACTIVATE_BUTTON_CLASS = 'border-red-500/30 bg-red-500/10 text-red-700 hover:bg-red-500/20 hover:text-red-800 dark:text-red-400 dark:hover:text-red-300'

export function CategoryPage() {
  const { categoryId } = useParams({ from: '/strategies/$categoryId' })
  const navigate = useNavigate()

  const [editingStrategy, setEditingStrategy] = useState<Strategy | null>(null)
  const [newStrategyOpen, setNewStrategyOpen] = useState(false)
  const [newStrategyName, setNewStrategyName] = useState('')
  const [newStrategyContent, setNewStrategyContent] = useState('')
  const [editingName, setEditingName] = useState('')
  const [editingContent, setEditingContent] = useState('')
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false)
  const [renameDialogOpen, setRenameDialogOpen] = useState(false)
  const [newCategoryName, setNewCategoryName] = useState('')
  const isInitialLoadRef = useRef(true)
  const skipNextAutosaveRef = useRef(false)

  const { config, loading, load, save, updateCategory, deleteCategory, addStrategy, updateStrategy, deleteStrategy, setActiveStrategy, clearActiveStrategy, clearAllActiveStrategies } = useConfigStore()
  const { restartIfConnected, notifyConfigApplied } = useConnectionStore()

  useEffect(() => {
    load().then(() => {
      isInitialLoadRef.current = false
    })
  }, [load])

  useEffect(() => {
    if (skipNextAutosaveRef.current) {
      skipNextAutosaveRef.current = false
      return
    }

    if (config && !isInitialLoadRef.current) {
      void save()
    }
  }, [config, save])

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

  const handleSetActive = async (strategyId: string) => {
    if (!categoryId)
      return

    try {
      skipNextAutosaveRef.current = true
      setActiveStrategy(categoryId, strategyId)
      await save()
      await restartIfConnected()
      notifyConfigApplied('Стратегия активирована')
    }
    catch (e) {
      toast.error(`Ошибка активации стратегии: ${e instanceof Error ? e.message : String(e)}`)
    }
  }

  const handleClearActive = async (strategyId: string) => {
    if (!categoryId)
      return

    try {
      skipNextAutosaveRef.current = true
      clearActiveStrategy(categoryId, strategyId)
      await save()
      await restartIfConnected()
      notifyConfigApplied('Стратегия деактивирована')
    }
    catch (e) {
      toast.error(`Ошибка деактивации стратегии: ${e instanceof Error ? e.message : String(e)}`)
    }
  }

  const handleClearAllActive = async () => {
    if (!categoryId)
      return

    try {
      skipNextAutosaveRef.current = true
      clearAllActiveStrategies(categoryId)
      await save()
      await restartIfConnected()
      notifyConfigApplied('Активные стратегии отключены')
    }
    catch (e) {
      toast.error(`Ошибка деактивации стратегий: ${e instanceof Error ? e.message : String(e)}`)
    }
  }

  const handleDeleteStrategy = async (strategyId: string) => {
    if (categoryId) {
      const strategy = category?.strategies.find(s => s.id === strategyId)
      const wasActive = strategy?.active ?? false

      if (wasActive) {
        deleteStrategy(categoryId, strategyId)
        try {
          skipNextAutosaveRef.current = true
          await save()
          await restartIfConnected()
          toast.success('Стратегия удалена')
        }
        catch (err) {
          console.error('Failed to save/restart after deleting strategy:', err)
          toast.error('Ошибка сохранения после удаления стратегии')
          skipNextAutosaveRef.current = false
          await load()
        }
      }
      else {
        deleteStrategy(categoryId, strategyId)
        toast.success('Стратегия удалена')
      }
    }
  }

  const handleDeleteCategory = async () => {
    if (categoryId) {
      const hadActiveStrategy = category?.strategies.some(s => s.active) ?? false
      skipNextAutosaveRef.current = true
      deleteCategory(categoryId)
      try {
        await save()
      }
      catch (err) {
        console.error('Failed to save after deleting category:', err)
        toast.error('Ошибка сохранения после удаления категории')
        skipNextAutosaveRef.current = false
        await load()
        return
      }
      if (hadActiveStrategy) {
        try {
          await restartIfConnected()
        }
        catch (err) {
          console.error('Failed to restart after deleting category:', err)
        }
      }
      setDeleteDialogOpen(false)
      toast.success('Категория удалена')
      navigate({ to: '/strategies' })
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
      <ScrollArea className="h-full">
        <div className="p-6 space-y-6">
          <Link to="/strategies" className="flex items-center gap-2 text-muted-foreground hover:text-foreground">
            <ArrowLeft className="w-4 h-4" />
            Назад к категориям
          </Link>
          <p className="text-muted-foreground">Категория не найдена</p>
        </div>
      </ScrollArea>
    )
  }

  return (
    <ScrollArea className="h-full">
      <div className="p-6 space-y-6">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-4">
            <Link to="/strategies" className="text-muted-foreground hover:text-foreground cursor-pointer" aria-label="Назад к категориям">
              <ArrowLeft className="w-5 h-5" />
            </Link>
            <div>
              <h1 className="text-2xl font-medium">{category.name}</h1>
              <p className="text-sm text-muted-foreground mt-1">
                {category.strategies.length}
                {' '}
                {(() => {
                  const n = category.strategies.length
                  const lastTwo = n % 100
                  const last = n % 10
                  if (lastTwo >= 11 && lastTwo <= 14)
                    return 'стратегий'
                  if (last === 1)
                    return 'стратегия'
                  if (last >= 2 && last <= 4)
                    return 'стратегии'
                  return 'стратегий'
                })()}
              </p>
            </div>
          </div>
          <div className="flex items-center gap-2">
            <Tooltip>
              <TooltipTrigger asChild>
                <Button size="icon" onClick={() => setNewStrategyOpen(true)} aria-label="Новая стратегия">
                  <Plus className="w-4 h-4" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>Новая стратегия</TooltipContent>
            </Tooltip>
            <Tooltip>
              <TooltipTrigger asChild>
                <Button variant="outline" size="icon" onClick={openRenameDialog} aria-label="Переименовать категорию">
                  <Pencil className="w-4 h-4" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>Переименовать категорию</TooltipContent>
            </Tooltip>
            {category.strategies.some(s => s.active) && (
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button
                    variant="outline"
                    size="icon"
                    className={DEACTIVATE_BUTTON_CLASS}
                    onClick={handleClearAllActive}
                    aria-label="Деактивировать текущую стратегию"
                  >
                    <BrushCleaning className="w-4 h-4" />
                  </Button>
                </TooltipTrigger>
                <TooltipContent>Деактивировать текущую стратегию</TooltipContent>
              </Tooltip>
            )}
            <AlertDialog open={deleteDialogOpen} onOpenChange={setDeleteDialogOpen}>
              <Tooltip>
                <TooltipTrigger asChild>
                  <AlertDialogTrigger asChild>
                    <Button
                      variant="ghost"
                      size="icon"
                      className="bg-red-500/10 text-red-600 hover:bg-red-500/20 hover:text-red-700 dark:text-red-400 dark:hover:text-red-300"
                      aria-label={`Удалить категорию ${category.name}`}
                    >
                      <Trash2 className="w-4 h-4" />
                    </Button>
                  </AlertDialogTrigger>
                </TooltipTrigger>
                <TooltipContent>Удалить категорию</TooltipContent>
              </Tooltip>
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
                        <span className="font-normal">{strategy.name}</span>
                        {strategy.active && (
                          <span className="flex h-5 w-5 items-center justify-center rounded bg-green-600 text-white" aria-hidden="true">
                            <Check className="h-3.5 w-3.5" />
                          </span>
                        )}
                        {strategy.active && <span className="sr-only">Активная стратегия</span>}
                      </div>
                      <div className="flex items-center gap-2">
                        {strategy.active
                          ? (
                              <Tooltip>
                                <TooltipTrigger asChild>
                                  <Button
                                    variant="outline"
                                    size="icon"
                                    className={DEACTIVATE_BUTTON_CLASS}
                                    onClick={() => handleClearActive(strategy.id)}
                                    aria-label={`Деактивировать стратегию ${strategy.name}`}
                                  >
                                    <BrushCleaning className="h-4 w-4" />
                                  </Button>
                                </TooltipTrigger>
                                <TooltipContent>Деактивировать</TooltipContent>
                              </Tooltip>
                            )
                          : (
                              <Tooltip>
                                <TooltipTrigger asChild>
                                  <Button
                                    variant="outline"
                                    size="icon"
                                    onClick={() => handleSetActive(strategy.id)}
                                    aria-label={`Активировать стратегию ${strategy.name}`}
                                  >
                                    <Check className="h-4 w-4" />
                                  </Button>
                                </TooltipTrigger>
                                <TooltipContent>Активировать</TooltipContent>
                              </Tooltip>
                            )}
                        <Tooltip>
                          <TooltipTrigger asChild>
                            <Button
                              variant="outline"
                              size="icon"
                              onClick={() => handleEditStrategy(strategy)}
                              aria-label={`Редактировать стратегию ${strategy.name}`}
                            >
                              <Pencil className="h-4 w-4" />
                            </Button>
                          </TooltipTrigger>
                          <TooltipContent>Редактировать</TooltipContent>
                        </Tooltip>
                        <Tooltip>
                          <TooltipTrigger asChild>
                            <Button
                              variant="ghost"
                              size="icon"
                              className="bg-red-500/10 text-red-600 hover:bg-red-500/20 hover:text-red-700 dark:text-red-400 dark:hover:text-red-300"
                              onClick={() => handleDeleteStrategy(strategy.id)}
                              aria-label={`Удалить стратегию ${strategy.name}`}
                            >
                              <Trash2 className="h-4 w-4" />
                            </Button>
                          </TooltipTrigger>
                          <TooltipContent>Удалить</TooltipContent>
                        </Tooltip>
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
              <div className="space-y-2">
                <Label htmlFor="strategy-name">Название стратегии</Label>
                <Input
                  id="strategy-name"
                  placeholder="Название стратегии"
                  value={newStrategyName}
                  onChange={e => setNewStrategyName(e.target.value)}
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="strategy-content">Содержимое</Label>
                <Textarea
                  id="strategy-content"
                  placeholder="--dpi-desync=fake&#10;--dpi-desync-autottl=2"
                  value={newStrategyContent}
                  onChange={e => setNewStrategyContent(e.target.value)}
                  rows={8}
                  className="font-mono text-sm"
                />
              </div>
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
              <div className="space-y-2">
                <Label htmlFor="edit-strategy-name">Название стратегии</Label>
                <Input
                  id="edit-strategy-name"
                  placeholder="Название стратегии"
                  value={editingName}
                  onChange={e => setEditingName(e.target.value)}
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="edit-strategy-content">Содержимое</Label>
                <Textarea
                  id="edit-strategy-content"
                  placeholder="--dpi-desync=fake&#10;--dpi-desync-autottl=2"
                  value={editingContent}
                  onChange={e => setEditingContent(e.target.value)}
                  rows={8}
                  className="font-mono text-sm"
                />
              </div>
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
              <div className="space-y-2">
                <Label htmlFor="category-name">Название категории</Label>
                <Input
                  id="category-name"
                  placeholder="Название категории"
                  value={newCategoryName}
                  onChange={e => setNewCategoryName(e.target.value)}
                  onKeyDown={e => e.key === 'Enter' && handleRenameCategory()}
                />
              </div>
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
    </ScrollArea>
  )
}
