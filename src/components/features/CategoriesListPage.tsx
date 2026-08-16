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
import { restrictToParentElement, restrictToVerticalAxis } from '@dnd-kit/modifiers'
import {
  SortableContext,
  sortableKeyboardCoordinates,
  useSortable,
  verticalListSortingStrategy,
} from '@dnd-kit/sortable'
import { CSS } from '@dnd-kit/utilities'
import { useNavigate } from '@tanstack/react-router'
import { BrushCleaning, ChevronRight, FilePenLine, FolderOpen, GripVertical, Loader2, Package, Pencil, Plus, RefreshCcw, RotateCcw, Trash2, UserRoundPlus } from 'lucide-react'
import { memo, useCallback, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
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
import { openStrategiesDirectory } from '@/lib/tauri'
import { useConfigStore } from '@/stores/config.store'
import { useConnectionStore } from '@/stores/connection.store'

interface SortableCategoryItemProps {
  category: Category
  config: AppConfig | null
  builtinConfig: AppConfig | null
  onClearActive: (categoryId: string, e: React.MouseEvent) => void
  onRename: (category: Category) => void
  onDelete: (category: Category) => void
  onRestoreSystem: (category: Category) => void
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

const SortableCategoryItem = memo(({ category, config, builtinConfig, onClearActive, onRename, onDelete, onRestoreSystem }: SortableCategoryItemProps) => {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const activeStrategies = category.strategies.filter(strategy => strategy.active)
  const activeCount = activeStrategies.length
  const activeStrategiesLabel = formatActiveStrategiesLabel(activeStrategies)
  const activeStrategiesSrId = `category-${category.id}-active-strategies`
  const builtinCategory = getBuiltinCategory(builtinConfig, category.id)
  const isSystem = isSystemCategory(category)
  const isModified = isSystemCategoryModified(category, config)
  const updateAvailable = builtinCategory ? isSystemCategoryUpdateAvailable(category, builtinCategory) : false
  const isLegacySystemCategory = isSystem && !builtinCategory

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

  const openCategory = () => {
    void navigate({ to: '/strategies/$categoryId', params: { categoryId: category.id } })
  }

  return (
    <div
      ref={setNodeRef}
      style={style}
      className="group relative flex h-[4.5rem] items-center gap-3 rounded-lg border bg-card px-4 py-3"
    >
      <button
        type="button"
        className="absolute inset-0 z-0 cursor-pointer rounded-lg"
        aria-label={t('categories.openAria', { name: category.name })}
        aria-describedby={activeStrategiesSrId}
        onClick={openCategory}
      />
      <button
        type="button"
        {...attributes}
        {...listeners}
        aria-label={t('categories.dragAria', { name: category.name })}
        aria-describedby={activeStrategiesSrId}
        className="text-muted-foreground hover:text-foreground relative z-20 flex size-9 shrink-0 cursor-grab touch-none items-center justify-center rounded-md border border-border/70 bg-muted/25 transition-colors active:cursor-grabbing"
      >
        <GripVertical className="size-4" />
      </button>
      <div
        className="pointer-events-none relative z-10 -my-3 flex min-w-0 flex-1 items-center gap-2 self-stretch py-3"
      >
        <div className="flex min-w-0 flex-1 items-center justify-between gap-3">
          <div className="min-w-0 flex-1 space-y-1">
            <div className="flex min-w-0 items-center gap-1">
              <span className="block min-w-0 shrink-0 truncate text-sm font-normal">{category.name}</span>
              <div className="flex min-w-0 items-center gap-1 text-muted-foreground">
                {isSystem
                  ? (
                      <InlineMarker icon={Package} label={t('categories.systemBadge')} className="pointer-events-auto" />
                    )
                  : (
                      <InlineMarker icon={UserRoundPlus} label={t('categories.customBadge')} className="pointer-events-auto text-primary/80" />
                    )}
                {isModified && (
                  <InlineMarker icon={FilePenLine} label={t('categories.modifiedBadge')} className="pointer-events-auto text-warning" />
                )}
                {builtinCategory && isSystem && (isModified || updateAvailable) && (
                  <InlineMarker
                    icon={updateAvailable ? RefreshCcw : RotateCcw}
                    label={updateAvailable
                      ? t('categories.updateAvailable')
                      : t('categories.rollbackToSystem')}
                    className={updateAvailable ? 'pointer-events-auto text-primary' : 'pointer-events-auto text-destructive'}
                    onClick={() => onRestoreSystem(category)}
                  />
                )}
                {isLegacySystemCategory && (
                  <InlineMarker
                    icon={RotateCcw}
                    label={t('categories.legacyBadge')}
                    className="pointer-events-auto text-warning"
                  />
                )}
                {activeCount > 0
                  ? (
                      activeStrategiesLabel && (
                        <Tooltip>
                          <TooltipTrigger asChild>
                            <span className="pointer-events-auto max-w-[14rem] cursor-help truncate text-xs text-success animate-pulse">
                              {activeStrategiesLabel}
                            </span>
                          </TooltipTrigger>
                          <TooltipContent>
                            {activeCount === 1 ? t('categories.currentActive') : formatActiveStrategiesSrText(activeCount)}
                          </TooltipContent>
                        </Tooltip>
                      )
                    )
                  : (
                      <Tooltip>
                        <TooltipTrigger asChild>
                          <span
                            className="pointer-events-auto inline-flex size-2 cursor-help rounded-full bg-destructive animate-pulse"
                            aria-hidden="true"
                          />
                        </TooltipTrigger>
                        <TooltipContent>{t('categories.noActive')}</TooltipContent>
                      </Tooltip>
                    )}
              </div>
            </div>
            <p className="text-xs text-muted-foreground">
              {formatStrategiesCount(category.strategies.length)}
            </p>
          </div>
        </div>
        <span id={activeStrategiesSrId} className="sr-only">
          {formatActiveStrategiesSrText(activeCount)}
        </span>
        <div className="-my-3 ml-auto flex shrink-0 self-stretch items-center rounded-md py-3 text-muted-foreground">
          <ChevronRight className="size-4 group-hover:translate-x-1 transition-transform" />
        </div>
      </div>
      <div className="relative z-20 flex items-center gap-1">
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="outline"
              size="icon"
              onClick={() => onRename(category)}
              className="cursor-pointer"
              aria-label={t('categories.renameAria', { name: category.name })}
            >
              <Pencil className="size-4" />
            </Button>
          </TooltipTrigger>
          <TooltipContent>{t('categories.renameTooltip')}</TooltipContent>
        </Tooltip>
        {activeCount > 0 && (
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                variant="outline"
                size="icon"
                onClick={e => onClearActive(category.id, e)}
                className="cursor-pointer text-warning hover:text-warning"
                aria-label={t('categories.clearStrategyAria')}
              >
                <BrushCleaning className="size-4" />
              </Button>
            </TooltipTrigger>
            <TooltipContent>{t('categories.deactivateTooltip')}</TooltipContent>
          </Tooltip>
        )}
        <AlertDialog>
          <Tooltip>
            <TooltipTrigger asChild>
              <AlertDialogTrigger asChild>
                <Button
                  variant="outline"
                  size="icon"
                  className="cursor-pointer text-destructive hover:text-destructive"
                  aria-label={t('categories.deleteAria', { name: category.name })}
                >
                  <Trash2 className="size-4" />
                </Button>
              </AlertDialogTrigger>
            </TooltipTrigger>
            <TooltipContent>{t('categories.deleteTooltip')}</TooltipContent>
          </Tooltip>
          <AlertDialogContent>
            <AlertDialogHeader>
              <AlertDialogTitle>{t('categories.deleteDialogTitle')}</AlertDialogTitle>
              <AlertDialogDescription>
                {t('categories.deleteDialogDescription', { name: category.name })}
              </AlertDialogDescription>
            </AlertDialogHeader>
            <AlertDialogFooter>
              <AlertDialogCancel>{t('common.cancel')}</AlertDialogCancel>
              <AlertDialogAction onClick={() => onDelete(category)} className="bg-destructive text-destructive-foreground hover:bg-destructive/90">
                {t('common.delete')}
              </AlertDialogAction>
            </AlertDialogFooter>
          </AlertDialogContent>
        </AlertDialog>
      </div>
    </div>
  )
})

export function CategoriesListPage() {
  const { t } = useTranslation()
  const [newCategoryOpen, setNewCategoryOpen] = useState(false)
  const [newCategoryName, setNewCategoryName] = useState('')
  const [renameDialogOpen, setRenameDialogOpen] = useState(false)
  const categoryToRenameRef = useRef<Category | null>(null)
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

  const handleAddCategory = useCallback(async () => {
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
      toast.success(t('categories.categoryAdded'))
    }
    catch (e) {
      revertTo(previousConfig)
      toast.error(`Ошибка сохранения категории: ${e instanceof Error ? e.message : String(e)}`)
    }
  }, [newCategoryName, addCategory, saveNow, addConfigLog, revertTo, t])

  const handleClearActive = useCallback(async (categoryId: string, e: React.MouseEvent) => {
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
      const category = currentConfig.categories.find(item => item.id === categoryId)
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
      notifyConfigApplied(t('categories.strategyDeactivated'))
    }
    catch (err) {
      console.error('Failed to restart after deactivating strategy:', err)
      notifyConfigApplied('Стратегия деактивирована, но не удалось переподключиться')
    }
  }, [clearAllActiveStrategies, saveNow, addConfigLog, revertTo, restartIfConnected, notifyConfigApplied, t])

  const handleRestoreSystemCategory = useCallback(async (category: Category) => {
    const currentConfig = useConfigStore.getState().config
    const currentBuiltinConfig = useConfigStore.getState().builtinConfig
    const builtinCategory = getBuiltinCategory(currentBuiltinConfig, category.id)
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
      notifyConfigApplied(t('categories.categoryUpdated'))
    }
    catch (error) {
      toast.error(`Категория обновлена, но не удалось применить изменения: ${error instanceof Error ? error.message : String(error)}`)
    }
    finally {
      setSystemCategoryTarget(null)
    }
  }, [restoreBuiltinCategory, saveNow, revertTo, addConfigLog, restartIfConnected, notifyConfigApplied, t])

  const handleDragEnd = useCallback(async (event: DragEndEvent) => {
    const { active, over } = event

    if (over && active.id !== over.id) {
      const currentConfig = useConfigStore.getState().config
      if (!currentConfig) {
        return
      }

      const previousConfig = structuredClone(currentConfig)
      const oldIndex = currentConfig.categories.findIndex(c => c.id === active.id) ?? -1
      const newIndex = currentConfig.categories.findIndex(c => c.id === over.id) ?? -1
      if (oldIndex !== -1 && newIndex !== -1) {
        reorderCategories(oldIndex, newIndex)
        try {
          await saveNow()
          addConfigLog('изменён порядок категорий')
          toast.success(t('categories.orderSaved'))
        }
        catch (e) {
          revertTo(previousConfig)
          toast.error(`Ошибка сохранения порядка категорий: ${e instanceof Error ? e.message : String(e)}`)
        }
      }
    }
  }, [reorderCategories, saveNow, addConfigLog, revertTo, t])

  const handleOpenRenameDialog = useCallback((category: Category) => {
    categoryToRenameRef.current = category
    setNewCategoryNameDraft(category.name)
    setRenameDialogOpen(true)
  }, [])

  const handleRenameCategory = useCallback(async () => {
    if (!categoryToRenameRef.current || !newCategoryNameDraft.trim()) {
      return
    }

    const currentConfig = useConfigStore.getState().config
    if (!currentConfig) {
      return
    }

    const previousConfig = structuredClone(currentConfig)
    const nextName = newCategoryNameDraft.trim()
    updateCategory(categoryToRenameRef.current.id, nextName)
    try {
      await saveNow()
      addConfigLog(`категория "${categoryToRenameRef.current.name}" переименована в "${nextName}"`)
      setRenameDialogOpen(false)
      categoryToRenameRef.current = null
      setNewCategoryNameDraft('')
      toast.success(t('categories.categoryRenamed'))
    }
    catch (e) {
      revertTo(previousConfig)
      toast.error(`Ошибка сохранения категории: ${e instanceof Error ? e.message : String(e)}`)
    }
  }, [newCategoryNameDraft, updateCategory, saveNow, addConfigLog, revertTo, t])

  const handleDeleteCategory = useCallback(async (category: Category) => {
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
    toast.success(t('categories.categoryDeleted'))
  }, [deleteCategory, saveNow, revertTo, restartIfConnected, addConfigLog, t])

  const systemCategoryBuiltin = systemCategoryTarget ? getBuiltinCategory(builtinConfig, systemCategoryTarget.id) : null
  const systemCategoryUpdateAvailable = systemCategoryTarget && systemCategoryBuiltin
    ? isSystemCategoryUpdateAvailable(systemCategoryTarget, systemCategoryBuiltin)
    : false

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <Loader2 className="size-6 animate-spin" />
      </div>
    )
  }

  return (
    <LenisScrollArea className="h-full min-h-0">
      <div className="p-6 space-y-6">
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-2xl font-medium">{t('categories.title')}</h1>
            <p className="text-sm text-muted-foreground mt-1">
              {t('categories.subtitle')}
            </p>
          </div>
          <div className="flex items-center gap-2">
            <Button
              variant="outline"
              onClick={() => {
                void openStrategiesDirectory().catch((err: unknown) => {
                  toast.error('Не удалось открыть папку', {
                    description: err instanceof Error ? err.message : String(err),
                  })
                })
              }}
            >
              <FolderOpen className="size-4" />
              {t('categories.strategiesFolder')}
            </Button>
            <Button onClick={() => setNewCategoryOpen(true)}>
              <Plus className="size-4" />
              {t('categories.newCategory')}
            </Button>
          </div>
        </div>

        <div className="space-y-3">
          {config?.categories.length === 0
            ? (
                <p className="text-sm text-muted-foreground">{t('categories.noCategories')}</p>
              )
            : (
                <DndContext
                  sensors={sensors}
                  collisionDetection={closestCenter}
                  modifiers={[restrictToVerticalAxis, restrictToParentElement]}
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
                        config={config}
                        builtinConfig={builtinConfig}
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
                {systemCategoryUpdateAvailable
                  ? t('categories.updateSystemDialogTitle')
                  : t('categories.restoreSystemDialogTitle')}
              </AlertDialogTitle>
              <AlertDialogDescription>
                {systemCategoryTarget
                  ? t('categories.updateSystemDialogDescription', { name: systemCategoryTarget.name })
                  : ''}
              </AlertDialogDescription>
            </AlertDialogHeader>
            <AlertDialogFooter>
              <AlertDialogCancel>{t('common.cancel')}</AlertDialogCancel>
              <AlertDialogAction
                onClick={async () => {
                  if (systemCategoryTarget && systemCategoryBuiltin) {
                    await handleRestoreSystemCategory(systemCategoryTarget)
                  }
                }}
              >
                {systemCategoryUpdateAvailable ? t('categories.updateDialogConfirm') : t('categories.restoreDialogConfirm')}
              </AlertDialogAction>
            </AlertDialogFooter>
          </AlertDialogContent>
        </AlertDialog>

        <Dialog open={newCategoryOpen} onOpenChange={setNewCategoryOpen}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>{t('categories.newCategory')}</DialogTitle>
            </DialogHeader>
            <div className="py-4">
              <label htmlFor="new-category-name" className="text-sm font-normal">{t('categories.categoryNameLabel')}</label>
              <Input
                id="new-category-name"
                placeholder={t('categories.categoryNameLabel')}
                value={newCategoryName}
                onChange={e => setNewCategoryName(e.target.value)}
                onKeyDown={e => e.key === 'Enter' && handleAddCategory()}
              />
            </div>
            <DialogFooter>
              <Button variant="outline" onClick={() => setNewCategoryOpen(false)}>
                {t('common.cancel')}
              </Button>
              <Button onClick={handleAddCategory}>{t('categories.createButton')}</Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>

        <Dialog open={renameDialogOpen} onOpenChange={setRenameDialogOpen}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>{t('categories.renameCategoryTitle')}</DialogTitle>
            </DialogHeader>
            <div className="py-4">
              <label htmlFor="rename-category-name" className="text-sm font-normal">{t('categories.categoryNameLabel')}</label>
              <Input
                id="rename-category-name"
                placeholder={t('categories.categoryNameLabel')}
                value={newCategoryNameDraft}
                onChange={e => setNewCategoryNameDraft(e.target.value)}
                onKeyDown={e => e.key === 'Enter' && handleRenameCategory()}
              />
            </div>
            <DialogFooter>
              <Button variant="outline" onClick={() => setRenameDialogOpen(false)}>
                {t('common.cancel')}
              </Button>
              <Button onClick={handleRenameCategory}>{t('common.save')}</Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      </div>
    </LenisScrollArea>
  )
}
