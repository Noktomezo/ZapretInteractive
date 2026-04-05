import type { Strategy } from '@/lib/types'
import { Link, useNavigate, useParams } from '@tanstack/react-router'
import { ArrowLeft, BrushCleaning, Check, Loader2, Pencil, Plus, Trash2 } from 'lucide-react'
import { useRef, useState } from 'react'
import { toast } from 'sonner'
import {
  AlertDialog,
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
import { LenisScrollArea } from '@/components/ui/lenis-scroll-area'
import { Textarea } from '@/components/ui/textarea'
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip'
import { useMountEffect } from '@/hooks/use-mount-effect'
import { autosizeTextarea, forwardTextareaWheelToScrollArea } from '@/lib/editor-scroll'
import { useConfigStore } from '@/stores/config.store'
import { useConnectionStore } from '@/stores/connection.store'

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
  const newStrategyContentTextareaRef = useRef<HTMLTextAreaElement | null>(null)
  const editStrategyContentTextareaRef = useRef<HTMLTextAreaElement | null>(null)
  const config = useConfigStore(state => state.config)
  const loading = useConfigStore(state => state.loading)
  const load = useConfigStore(state => state.load)
  const reload = useConfigStore(state => state.reload)
  const saveNow = useConfigStore(state => state.saveNow)
  const revertTo = useConfigStore(state => state.revertTo)
  const updateCategory = useConfigStore(state => state.updateCategory)
  const deleteCategory = useConfigStore(state => state.deleteCategory)
  const addStrategy = useConfigStore(state => state.addStrategy)
  const updateStrategy = useConfigStore(state => state.updateStrategy)
  const deleteStrategy = useConfigStore(state => state.deleteStrategy)
  const setActiveStrategy = useConfigStore(state => state.setActiveStrategy)
  const clearActiveStrategy = useConfigStore(state => state.clearActiveStrategy)
  const clearAllActiveStrategies = useConfigStore(state => state.clearAllActiveStrategies)
  const restartIfConnected = useConnectionStore(state => state.restartIfConnected)
  const notifyConfigApplied = useConnectionStore(state => state.notifyConfigApplied)
  const addConfigLog = useConnectionStore(state => state.addConfigLog)

  useMountEffect(() => {
    void load()
  })

  const category = config?.categories.find(c => c.id === categoryId)

  const handleAddStrategy = async () => {
    if (!newStrategyName.trim() || !newStrategyContent.trim() || !categoryId) {
      return
    }

    const currentConfig = useConfigStore.getState().config
    if (!currentConfig) {
      return
    }

    const previousConfig = structuredClone(currentConfig)
    const strategyName = newStrategyName.trim()
    addStrategy(categoryId, strategyName, newStrategyContent.trim())
    try {
      await saveNow()
      if (category) {
        addConfigLog(`добавлена стратегия "${strategyName}" в категории "${category.name}"`)
      }
      setNewStrategyName('')
      setNewStrategyContent('')
      setNewStrategyOpen(false)
      toast.success('Стратегия добавлена')
    }
    catch (e) {
      revertTo(previousConfig)
      toast.error(`Ошибка сохранения стратегии: ${e instanceof Error ? e.message : String(e)}`)
    }
  }

  const handleEditStrategy = (strategy: Strategy) => {
    setEditingStrategy(strategy)
    setEditingName(strategy.name)
    setEditingContent(strategy.content)
    requestAnimationFrame(() => autosizeTextarea(editStrategyContentTextareaRef.current))
  }

  const handleSaveEdit = async () => {
    if (!editingStrategy || !categoryId) {
      return
    }

    const currentConfig = useConfigStore.getState().config
    if (!currentConfig) {
      return
    }

    const previousConfig = structuredClone(currentConfig)
    const previousName = editingStrategy.name
    const nextName = editingName.trim()
    updateStrategy(categoryId, editingStrategy.id, {
      name: nextName,
      content: editingContent,
    })
    try {
      await saveNow()
      if (category) {
        addConfigLog(
          previousName !== nextName
            ? `стратегия "${previousName}" переименована в "${nextName}" в категории "${category.name}"`
            : `обновлена стратегия "${previousName}" в категории "${category.name}"`,
        )
      }
      setEditingStrategy(null)
      toast.success('Стратегия сохранена')
    }
    catch (e) {
      revertTo(previousConfig)
      toast.error(`Ошибка сохранения стратегии: ${e instanceof Error ? e.message : String(e)}`)
    }
  }

  const handleSetActive = async (strategyId: string) => {
    if (!categoryId)
      return

    try {
      setActiveStrategy(categoryId, strategyId)
      await saveNow()
      if (category) {
        const strategy = category.strategies.find(item => item.id === strategyId)
        if (strategy) {
          addConfigLog(`стратегия "${strategy.name}" активирована в категории "${category.name}"`)
        }
      }
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
      clearActiveStrategy(categoryId, strategyId)
      await saveNow()
      if (category) {
        const strategy = category.strategies.find(item => item.id === strategyId)
        if (strategy) {
          addConfigLog(`стратегия "${strategy.name}" деактивирована в категории "${category.name}"`)
        }
      }
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
      clearAllActiveStrategies(categoryId)
      await saveNow()
      if (category) {
        addConfigLog(`все активные стратегии отключены в категории "${category.name}"`)
      }
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
          await saveNow()
          if (category && strategy) {
            addConfigLog(`удалена стратегия "${strategy.name}" из категории "${category.name}"`)
          }
          toast.success('Стратегия удалена')
        }
        catch (err) {
          console.error('Failed to save after deleting strategy:', err)
          toast.error('Ошибка сохранения после удаления стратегии')
          await reload().catch(() => {})
          return
        }
        try {
          await restartIfConnected()
        }
        catch (err) {
          console.error('Failed to restart after deleting strategy:', err)
          toast.error('Стратегия удалена, но не удалось применить изменения к активному подключению', {
            description: err instanceof Error ? err.message : String(err),
            duration: 8000,
          })
        }
      }
      else {
        const currentConfig = useConfigStore.getState().config
        if (!currentConfig) {
          return
        }

        const previousConfig = structuredClone(currentConfig)
        deleteStrategy(categoryId, strategyId)
        try {
          await saveNow()
          if (category && strategy) {
            addConfigLog(`удалена стратегия "${strategy.name}" из категории "${category.name}"`)
          }
          toast.success('Стратегия удалена')
        }
        catch (e) {
          revertTo(previousConfig)
          toast.error(`Ошибка сохранения после удаления стратегии: ${e instanceof Error ? e.message : String(e)}`)
        }
      }
    }
  }

  const handleDeleteCategory = async () => {
    if (categoryId) {
      const hadActiveStrategy = category?.strategies.some(s => s.active) ?? false
      const categoryName = category?.name
      deleteCategory(categoryId)
      try {
        await saveNow()
      }
      catch (err) {
        console.error('Failed to save after deleting category:', err)
        toast.error('Ошибка сохранения после удаления категории')
        await reload().catch(() => {})
        return
      }
      if (hadActiveStrategy) {
        try {
          await restartIfConnected()
        }
        catch (err) {
          console.error('Failed to restart after deleting category:', err)
          toast.error('Категория удалена, но не удалось применить изменения к активному подключению', {
            description: err instanceof Error ? err.message : String(err),
            duration: 8000,
          })
        }
      }
      if (categoryName) {
        addConfigLog(`удалена категория "${categoryName}"`)
      }
      setDeleteDialogOpen(false)
      toast.success('Категория удалена')
      navigate({ to: '/strategies' })
    }
  }

  const handleRenameCategory = async () => {
    if (!categoryId || !newCategoryName.trim()) {
      return
    }

    const currentConfig = useConfigStore.getState().config
    if (!currentConfig) {
      return
    }

    const previousConfig = structuredClone(currentConfig)
    const previousName = category?.name
    const nextName = newCategoryName.trim()
    updateCategory(categoryId, nextName)
    try {
      await saveNow()
      if (previousName) {
        addConfigLog(`категория "${previousName}" переименована в "${nextName}"`)
      }
      setRenameDialogOpen(false)
      toast.success('Категория переименована')
    }
    catch (e) {
      revertTo(previousConfig)
      toast.error(`Ошибка сохранения категории: ${e instanceof Error ? e.message : String(e)}`)
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
      <LenisScrollArea className="h-full min-h-0">
        <div className="p-6 space-y-6">
          <Link to="/strategies" className="flex items-center gap-2 text-muted-foreground hover:text-foreground">
            <ArrowLeft className="w-4 h-4" />
            Назад к категориям
          </Link>
          <p className="text-muted-foreground">Категория не найдена</p>
        </div>
      </LenisScrollArea>
    )
  }

  return (
    <LenisScrollArea className="h-full min-h-0">
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
          <div className="flex items-center gap-1">
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
                    className="border-warning/30 bg-warning/12 text-warning hover:bg-warning/18"
                    onClick={handleClearAllActive}
                    aria-label="Деактивировать все активные стратегии"
                  >
                    <BrushCleaning className="w-4 h-4" />
                  </Button>
                </TooltipTrigger>
                <TooltipContent>Деактивировать все активные стратегии</TooltipContent>
              </Tooltip>
            )}
            <AlertDialog open={deleteDialogOpen} onOpenChange={setDeleteDialogOpen}>
              <Tooltip>
                <TooltipTrigger asChild>
                  <AlertDialogTrigger asChild>
                    <Button
                      variant="outline"
                      size="icon"
                      className="border-destructive/30 bg-destructive/10 text-destructive hover:bg-destructive/18"
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
                  <Button
                    onClick={async () => {
                      await handleDeleteCategory()
                    }}
                    className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
                  >
                    Удалить
                  </Button>
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
                    className={strategy.active
                      ? 'border border-success/30 bg-success/10 rounded-lg p-4 space-y-3'
                      : 'border border-border rounded-lg p-4 space-y-3'}
                  >
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-3">
                        <span className="font-normal">{strategy.name}</span>
                        {strategy.active && (
                          <span className="flex h-4 w-4 items-center justify-center rounded bg-success text-white dark:text-background" aria-hidden="true">
                            <Check className="h-3 w-3" />
                          </span>
                        )}
                        {strategy.active && <span className="sr-only">Активная стратегия</span>}
                      </div>
                      <div className="flex items-center gap-1">
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
                        {strategy.active
                          ? (
                              <Tooltip>
                                <TooltipTrigger asChild>
                                  <Button
                                    variant="outline"
                                    size="icon"
                                    className="border-warning/35 bg-warning/14 text-warning hover:bg-warning/22"
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
                              className="border-destructive/30 bg-destructive/10 text-destructive hover:bg-destructive/18"
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
                    <pre className="text-xs text-muted-foreground bg-muted p-3 rounded-md overflow-x-auto">
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
                <LenisScrollArea
                  className="max-h-[calc(100vh-22rem)] rounded-md border border-border/80 bg-background/92 shadow-xs transition-[border-color,box-shadow,background-color] hover:border-border focus-within:border-ring focus-within:ring-[3px] focus-within:ring-ring/50 dark:bg-input/30"
                  contentClassName="cursor-text"
                  onClick={() => newStrategyContentTextareaRef.current?.focus()}
                >
                  <Textarea
                    data-lenis-prevent
                    ref={newStrategyContentTextareaRef}
                    id="strategy-content"
                    placeholder="--dpi-desync=fake&#10;--dpi-desync-autottl=2"
                    value={newStrategyContent}
                    onChange={(e) => {
                      setNewStrategyContent(e.target.value)
                      autosizeTextarea(e.currentTarget)
                    }}
                    onWheel={forwardTextareaWheelToScrollArea}
                    rows={10}
                    className="resize-none overflow-hidden rounded-none border-0 bg-transparent px-3 py-3 font-mono text-sm shadow-none hover:border-transparent focus-visible:border-transparent focus-visible:ring-0"
                  />
                </LenisScrollArea>
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
                <LenisScrollArea
                  className="max-h-[calc(100vh-22rem)] rounded-md border border-border/80 bg-background/92 shadow-xs transition-[border-color,box-shadow,background-color] hover:border-border focus-within:border-ring focus-within:ring-[3px] focus-within:ring-ring/50 dark:bg-input/30"
                  contentClassName="cursor-text"
                  onClick={() => editStrategyContentTextareaRef.current?.focus()}
                >
                  <Textarea
                    data-lenis-prevent
                    ref={editStrategyContentTextareaRef}
                    id="edit-strategy-content"
                    placeholder="--dpi-desync=fake&#10;--dpi-desync-autottl=2"
                    value={editingContent}
                    onChange={(e) => {
                      setEditingContent(e.target.value)
                      autosizeTextarea(e.currentTarget)
                    }}
                    onWheel={forwardTextareaWheelToScrollArea}
                    rows={10}
                    className="resize-none overflow-hidden rounded-none border-0 bg-transparent px-3 py-3 font-mono text-sm shadow-none hover:border-transparent focus-visible:border-transparent focus-visible:ring-0"
                  />
                </LenisScrollArea>
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
    </LenisScrollArea>
  )
}
