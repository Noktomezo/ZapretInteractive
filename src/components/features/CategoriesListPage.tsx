import type {
  DragEndEvent,
} from '@dnd-kit/core'
import type { AppConfig, Category } from '@/lib/types'
import {
  closestCenter,
  DndContext,
  KeyboardSensor,
  PointerSensor,
  useSensor,
  useSensors,
} from '@dnd-kit/core'
import {
  SortableContext,
  sortableKeyboardCoordinates,
  useSortable,
  verticalListSortingStrategy,
} from '@dnd-kit/sortable'
import { CSS } from '@dnd-kit/utilities'
import { Link } from '@tanstack/react-router'
import { BrushCleaning, ChevronRight, FilePenLine, GripVertical, Loader2, Package, Pencil, Plus, RefreshCcw, RotateCcw, Trash2, UserRoundPlus } from 'lucide-react'
import { useState } from 'react'
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
import { InlineMarker } from '@/components/ui/inline-marker'
import { Input } from '@/components/ui/input'
import { LenisScrollArea } from '@/components/ui/lenis-scroll-area'
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip'
import { useMountEffect } from '@/hooks/use-mount-effect'
import { buildRestoredCategory, getBuiltinCategory, isSystemCategory, isSystemCategoryModified, isSystemCategoryUpdateAvailable } from '@/lib/system-config'
import { useConfigStore } from '@/stores/config.store'
import { useConnectionStore } from '@/stores/connection.store'

interface SortableCategoryItemProps {
  category: Category
  config: CategoryListConfigContext
  onClearActive: (categoryId: string, e: React.MouseEvent) => void
  onRename: (category: Category) => void
  onDelete: (category: Category) => void
  onRestoreSystem: (category: Category) => void
}

interface CategoryListConfigContext {
  config: AppConfig | null
  builtinConfig: AppConfig | null
}

function formatStrategiesCount(count: number) {
  const lastTwoDigits = count % 100
  if (lastTwoDigits >= 11 && lastTwoDigits <= 14)
    return `${count} стратегий`

  const lastDigit = count % 10
  if (lastDigit === 1)
    return `${count} стратегия`
  if (lastDigit >= 2 && lastDigit <= 4)
    return `${count} стратегии`
  return `${count} стратегий`
}

function formatActiveStrategiesLabel(activeStrategies: Category['strategies']) {
  const activeCount = activeStrategies.length
  const firstActiveStrategy = activeStrategies[0]

  if (activeCount === 0 || !firstActiveStrategy) {
    return null
  }

  if (activeCount === 1) {
    return firstActiveStrategy.name
  }

  return `${firstActiveStrategy.name} +${activeCount - 1}`
}

function formatActiveStrategiesSrText(activeCount: number) {
  if (activeCount === 0) {
    return 'Нет активных стратегий'
  }

  const lastTwoDigits = activeCount % 100
  if (lastTwoDigits >= 11 && lastTwoDigits <= 14) {
    return `${activeCount} активных стратегий`
  }

  const lastDigit = activeCount % 10
  if (lastDigit === 1) {
    return `${activeCount} активная стратегия`
  }
  if (lastDigit >= 2 && lastDigit <= 4) {
    return `${activeCount} активные стратегии`
  }
  return `${activeCount} активных стратегий`
}

function SortableCategoryItem({ category, config, onClearActive, onRename, onDelete, onRestoreSystem }: SortableCategoryItemProps) {
  const activeStrategies = category.strategies.filter(strategy => strategy.active)
  const activeCount = activeStrategies.length
  const activeStrategiesLabel = formatActiveStrategiesLabel(activeStrategies)
  const builtinCategory = getBuiltinCategory(config.builtinConfig, category.id)

  const {
    attributes,
    listeners,
    setNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({ id: category.id })

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
    opacity: isDragging ? 0.5 : 1,
  }

  return (
    <div
      ref={setNodeRef}
      style={style}
      className="group flex h-20 items-center gap-3 rounded-lg border bg-card p-4"
    >
      <button
        {...attributes}
        {...listeners}
        className="cursor-grab active:cursor-grabbing text-muted-foreground hover:text-foreground touch-none"
      >
        <GripVertical className="w-4 h-4" />
      </button>
      <Link
        to="/strategies/$categoryId"
        params={{ categoryId: category.id }}
        className="-my-4 flex min-w-0 flex-1 self-stretch cursor-pointer items-center justify-between rounded-md py-4"
      >
        <div className="flex min-w-0 items-center gap-3">
          <div className="min-w-0 space-y-1">
            <div className="flex items-center gap-3">
              <span className="truncate text-sm font-normal">{category.name}</span>
              <div className="flex items-center gap-1 text-muted-foreground">
                {isSystemCategory(category)
                  ? (
                      <InlineMarker icon={Package} label="Системная категория" />
                    )
                  : (
                      <InlineMarker icon={UserRoundPlus} label="Пользовательская категория" className="text-primary/80" />
                    )}
                {isSystemCategoryModified(category, config.config) && (
                  <InlineMarker icon={FilePenLine} label="Системная категория изменена пользователем" className="text-warning" />
                )}
                {activeCount > 0
                  ? (
                      activeStrategiesLabel && (
                        <Tooltip>
                          <TooltipTrigger asChild>
                            <span className="max-w-[14rem] cursor-help truncate text-xs text-success animate-pulse">
                              {activeStrategiesLabel}
                            </span>
                          </TooltipTrigger>
                          <TooltipContent>
                            {activeCount === 1 ? 'Текущая активная стратегия' : formatActiveStrategiesSrText(activeCount)}
                          </TooltipContent>
                        </Tooltip>
                      )
                    )
                  : (
                      <Tooltip>
                        <TooltipTrigger asChild>
                          <span
                            className="inline-flex h-2 w-2 cursor-help rounded-full bg-destructive animate-pulse"
                            aria-hidden="true"
                          />
                        </TooltipTrigger>
                        <TooltipContent>Нет активных стратегий</TooltipContent>
                      </Tooltip>
                    )}
              </div>
              <span className="sr-only">
                {formatActiveStrategiesSrText(activeCount)}
              </span>
            </div>
            <p className="text-xs text-muted-foreground">
              {formatStrategiesCount(category.strategies.length)}
            </p>
          </div>
        </div>
        <div className="flex shrink-0 items-center text-muted-foreground">
          <ChevronRight className="w-4 h-4 group-hover:translate-x-1 transition-transform" />
        </div>
      </Link>
      <div className="flex items-center gap-1">
        {isSystemCategory(category) && (isSystemCategoryModified(category, config.config) || isSystemCategoryUpdateAvailable(category, builtinCategory)) && (
          <InlineMarker
            icon={isSystemCategoryUpdateAvailable(category, builtinCategory) ? RefreshCcw : RotateCcw}
            label={isSystemCategoryUpdateAvailable(category, builtinCategory)
              ? 'Обновить категорию до актуального системного значения'
              : 'Откатить категорию к системному значению'}
            className={isSystemCategoryUpdateAvailable(category, builtinCategory) ? 'text-primary' : 'text-destructive'}
            onClick={() => onRestoreSystem(category)}
          />
        )}
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="outline"
              size="icon"
              onClick={() => onRename(category)}
              className="cursor-pointer"
              aria-label={`Переименовать категорию ${category.name}`}
            >
              <Pencil className="w-4 h-4" />
            </Button>
          </TooltipTrigger>
          <TooltipContent>Переименовать</TooltipContent>
        </Tooltip>
        {activeCount > 0 && (
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                variant="outline"
                size="icon"
                onClick={e => onClearActive(category.id, e)}
                className="cursor-pointer border-warning/35 bg-warning/14 text-warning hover:bg-warning/22"
                aria-label="Очистить стратегию"
              >
                <BrushCleaning className="w-4 h-4" />
              </Button>
            </TooltipTrigger>
            <TooltipContent>Деактивировать текущую стратегию</TooltipContent>
          </Tooltip>
        )}
        <AlertDialog>
          <Tooltip>
            <TooltipTrigger asChild>
              <AlertDialogTrigger asChild>
                <Button
                  variant="outline"
                  size="icon"
                  className="cursor-pointer border-destructive/30 bg-destructive/10 text-destructive hover:bg-destructive/18"
                  aria-label={`Удалить категорию ${category.name}`}
                >
                  <Trash2 className="w-4 h-4" />
                </Button>
              </AlertDialogTrigger>
            </TooltipTrigger>
            <TooltipContent>Удалить</TooltipContent>
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
              <AlertDialogAction onClick={() => onDelete(category)} className="bg-destructive text-destructive-foreground hover:bg-destructive/90">
                Удалить
              </AlertDialogAction>
            </AlertDialogFooter>
          </AlertDialogContent>
        </AlertDialog>
      </div>
    </div>
  )
}

export function CategoriesListPage() {
  const [newCategoryOpen, setNewCategoryOpen] = useState(false)
  const [newCategoryName, setNewCategoryName] = useState('')
  const [renameDialogOpen, setRenameDialogOpen] = useState(false)
  const [categoryToRename, setCategoryToRename] = useState<Category | null>(null)
  const [newCategoryNameDraft, setNewCategoryNameDraft] = useState('')
  const [systemCategoryTarget, setSystemCategoryTarget] = useState<Category | null>(null)
  const config = useConfigStore(state => state.config)
  const builtinConfig = useConfigStore(state => state.builtinConfig)
  const loading = useConfigStore(state => state.loading)
  const load = useConfigStore(state => state.load)
  const saveNow = useConfigStore(state => state.saveNow)
  const addCategory = useConfigStore(state => state.addCategory)
  const revertTo = useConfigStore(state => state.revertTo)
  const updateCategory = useConfigStore(state => state.updateCategory)
  const restoreBuiltinCategory = useConfigStore(state => state.restoreBuiltinCategory)
  const deleteCategory = useConfigStore(state => state.deleteCategory)
  const clearAllActiveStrategies = useConfigStore(state => state.clearAllActiveStrategies)
  const reorderCategories = useConfigStore(state => state.reorderCategories)
  const restartIfConnected = useConnectionStore(state => state.restartIfConnected)
  const notifyConfigApplied = useConnectionStore(state => state.notifyConfigApplied)
  const addConfigLog = useConnectionStore(state => state.addConfigLog)

  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: {
        distance: 8,
      },
    }),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
    }),
  )

  useMountEffect(() => {
    void load()
  })

  const handleAddCategory = async () => {
    if (!newCategoryName.trim()) {
      return
    }

    const currentConfig = useConfigStore.getState().config
    if (!currentConfig) {
      return
    }

    const previousConfig = structuredClone(currentConfig)
    const categoryName = newCategoryName.trim()
    addCategory(categoryName)
    try {
      await saveNow()
      addConfigLog(`добавлена категория "${categoryName}"`)
      setNewCategoryName('')
      setNewCategoryOpen(false)
      toast.success('Категория добавлена')
    }
    catch (e) {
      revertTo(previousConfig)
      toast.error(`Ошибка сохранения категории: ${e instanceof Error ? e.message : String(e)}`)
    }
  }

  const handleClearActive = async (categoryId: string, e: React.MouseEvent) => {
    e.preventDefault()
    e.stopPropagation()
    const currentConfig = useConfigStore.getState().config
    if (!currentConfig) {
      return
    }

    const previousConfig = structuredClone(currentConfig)
    try {
      clearAllActiveStrategies(categoryId)
      await saveNow()
      const category = config?.categories.find(item => item.id === categoryId)
      if (category) {
        addConfigLog(`активные стратегии отключены в категории "${category.name}"`)
      }
    }
    catch (err) {
      revertTo(previousConfig)
      console.error('Failed to save after deactivating strategy:', err)
      toast.error('Ошибка сохранения после деактивации стратегии')
      return
    }
    try {
      await restartIfConnected()
      notifyConfigApplied('Стратегия деактивирована')
    }
    catch (err) {
      console.error('Failed to restart after deactivating strategy:', err)
      notifyConfigApplied('Стратегия деактивирована, но не удалось переподключиться')
    }
  }

  const handleRestoreSystemCategory = async (category: Category) => {
    const currentConfig = useConfigStore.getState().config
    const builtinCategory = getBuiltinCategory(builtinConfig, category.id)
    if (!currentConfig || !builtinCategory) {
      return
    }

    const previousConfig = structuredClone(currentConfig)
    restoreBuiltinCategory(category.id, buildRestoredCategory(category, builtinCategory))
    try {
      await saveNow()
    }
    catch (error) {
      revertTo(previousConfig)
      toast.error(`Ошибка обновления категории: ${error instanceof Error ? error.message : String(error)}`)
      return
    }

    try {
      addConfigLog(`категория "${category.name}" обновлена до системного значения`)
      await restartIfConnected()
      notifyConfigApplied('Категория обновлена')
    }
    catch (error) {
      toast.error(`Категория обновлена, но не удалось применить изменения: ${error instanceof Error ? error.message : String(error)}`)
    }
    finally {
      setSystemCategoryTarget(null)
    }
  }

  const handleDragEnd = async (event: DragEndEvent) => {
    const { active, over } = event

    if (over && active.id !== over.id) {
      const currentConfig = useConfigStore.getState().config
      if (!currentConfig) {
        return
      }

      const previousConfig = structuredClone(currentConfig)
      const oldIndex = config?.categories.findIndex(c => c.id === active.id) ?? -1
      const newIndex = config?.categories.findIndex(c => c.id === over.id) ?? -1
      if (oldIndex !== -1 && newIndex !== -1) {
        reorderCategories(oldIndex, newIndex)
        try {
          await saveNow()
          addConfigLog('изменён порядок категорий')
          toast.success('Порядок категорий сохранён')
        }
        catch (e) {
          revertTo(previousConfig)
          toast.error(`Ошибка сохранения порядка категорий: ${e instanceof Error ? e.message : String(e)}`)
        }
      }
    }
  }

  const handleOpenRenameDialog = (category: Category) => {
    setCategoryToRename(category)
    setNewCategoryNameDraft(category.name)
    setRenameDialogOpen(true)
  }

  const handleRenameCategory = async () => {
    if (!categoryToRename || !newCategoryNameDraft.trim()) {
      return
    }

    const currentConfig = useConfigStore.getState().config
    if (!currentConfig) {
      return
    }

    const previousConfig = structuredClone(currentConfig)
    const nextName = newCategoryNameDraft.trim()
    updateCategory(categoryToRename.id, nextName)
    try {
      await saveNow()
      addConfigLog(`категория "${categoryToRename.name}" переименована в "${nextName}"`)
      setRenameDialogOpen(false)
      setCategoryToRename(null)
      setNewCategoryNameDraft('')
      toast.success('Категория переименована')
    }
    catch (e) {
      revertTo(previousConfig)
      toast.error(`Ошибка сохранения категории: ${e instanceof Error ? e.message : String(e)}`)
    }
  }

  const handleDeleteCategory = async (category: Category) => {
    const currentConfig = useConfigStore.getState().config
    if (!currentConfig) {
      return
    }

    const previousConfig = structuredClone(currentConfig)
    const hadActiveStrategy = category.strategies.some(s => s.active)
    deleteCategory(category.id)
    try {
      await saveNow()
    }
    catch (err) {
      revertTo(previousConfig)
      console.error('Failed to save after deleting category:', err)
      toast.error('Ошибка сохранения после удаления категории')
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
    addConfigLog(`удалена категория "${category.name}"`)
    toast.success('Категория удалена')
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <Loader2 className="w-6 h-6 animate-spin" />
      </div>
    )
  }

  return (
    <LenisScrollArea className="h-full min-h-0">
      <div className="p-6 space-y-6">
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-2xl font-medium">Категории</h1>
            <p className="text-sm text-muted-foreground mt-1">
              Выберите категорию для управления стратегиями
            </p>
          </div>
          <Button onClick={() => setNewCategoryOpen(true)}>
            <Plus className="w-4 h-4" />
            Новая категория
          </Button>
        </div>

        <div className="space-y-3">
          {config?.categories.length === 0
            ? (
                <p className="text-sm text-muted-foreground">Нет категорий</p>
              )
            : (
                <DndContext
                  sensors={sensors}
                  collisionDetection={closestCenter}
                  onDragEnd={handleDragEnd}
                >
                  <SortableContext
                    items={config?.categories.map(c => c.id) ?? []}
                    strategy={verticalListSortingStrategy}
                  >
                    {config?.categories.map((category: Category) => (
                      <SortableCategoryItem
                        key={category.id}
                        category={category}
                        config={{ config, builtinConfig }}
                        onClearActive={handleClearActive}
                        onRename={handleOpenRenameDialog}
                        onDelete={handleDeleteCategory}
                        onRestoreSystem={setSystemCategoryTarget}
                      />
                    ))}
                  </SortableContext>
                </DndContext>
              )}
        </div>

        <AlertDialog open={!!systemCategoryTarget} onOpenChange={open => !open && setSystemCategoryTarget(null)}>
          <AlertDialogContent>
            <AlertDialogHeader>
              <AlertDialogTitle>
                {systemCategoryTarget && isSystemCategoryUpdateAvailable(systemCategoryTarget, getBuiltinCategory(builtinConfig, systemCategoryTarget.id))
                  ? 'Обновить системную категорию?'
                  : 'Откатить категорию к системному значению?'}
              </AlertDialogTitle>
              <AlertDialogDescription>
                {systemCategoryTarget
                  ? `Категория «${systemCategoryTarget.name}» будет возвращена к актуальному системному значению. Пользовательские изменения внутри категории будут сброшены.`
                  : ''}
              </AlertDialogDescription>
            </AlertDialogHeader>
            <AlertDialogFooter>
              <AlertDialogCancel>Отмена</AlertDialogCancel>
              <AlertDialogAction
                onClick={async () => {
                  if (systemCategoryTarget) {
                    await handleRestoreSystemCategory(systemCategoryTarget)
                  }
                }}
              >
                Обновить
              </AlertDialogAction>
            </AlertDialogFooter>
          </AlertDialogContent>
        </AlertDialog>

        <Dialog open={newCategoryOpen} onOpenChange={setNewCategoryOpen}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Новая категория</DialogTitle>
            </DialogHeader>
            <div className="py-4">
              <label htmlFor="new-category-name" className="text-sm font-normal">Название категории</label>
              <Input
                id="new-category-name"
                placeholder="Название категории"
                value={newCategoryName}
                onChange={e => setNewCategoryName(e.target.value)}
                onKeyDown={e => e.key === 'Enter' && handleAddCategory()}
              />
            </div>
            <DialogFooter>
              <Button variant="outline" onClick={() => setNewCategoryOpen(false)}>
                Отмена
              </Button>
              <Button onClick={handleAddCategory}>Создать</Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>

        <Dialog open={renameDialogOpen} onOpenChange={setRenameDialogOpen}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Переименовать категорию</DialogTitle>
            </DialogHeader>
            <div className="py-4">
              <label htmlFor="rename-category-name" className="text-sm font-normal">Название категории</label>
              <Input
                id="rename-category-name"
                placeholder="Название категории"
                value={newCategoryNameDraft}
                onChange={e => setNewCategoryNameDraft(e.target.value)}
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
    </LenisScrollArea>
  )
}
