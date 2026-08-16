import type { Strategy } from '@/lib/types'
import { Link, useNavigate, useParams } from '@tanstack/react-router'
import { ArrowLeft, BrushCleaning, Check, FilePenLine, FolderOpen, Loader2, Package, Pencil, Plus, RefreshCcw, RotateCcw, Trash2, UserRoundPlus } from 'lucide-react'
import { memo, useCallback, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
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
import { EditorTextarea } from '@/components/ui/editor-textarea'
import { InlineMarker } from '@/components/ui/inline-marker'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { LenisScrollArea } from '@/components/ui/lenis-scroll-area'
import { ScrollTopButton } from '@/components/ui/scroll-top-button'
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip'
import { useMountEffect } from '@/hooks/use-mount-effect'
import { autosizeTextarea } from '@/lib/editor-scroll'
import { buildRestoredCategory, buildRestoredStrategy, getBuiltinCategory, getBuiltinStrategy, isSystemCategory, isSystemCategoryModified, isSystemCategoryUpdateAvailable, isSystemStrategy, isSystemStrategyModified, isSystemStrategyUpdateAvailable } from '@/lib/system-config'
import { openStrategiesDirectory } from '@/lib/tauri'
import { cn } from '@/lib/utils'
import { useConfigStore } from '@/stores/config.store'
import { useConnectionStore } from '@/stores/connection.store'

const CRLF_REGEX = /\r\n/g

function normalizeStrategyText(value: string) {
  return value.replace(CRLF_REGEX, '\n').trim()
}

function formatActiveStrategiesLabel(activeStrategies: Strategy[]) {
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

function getStrategyDuplicateError(
  strategies: Strategy[],
  name: string,
  content: string,
  excludedStrategyId?: string,
) {
  const trimmedName = name.trim().toLocaleLowerCase()
  const normalizedContent = normalizeStrategyText(content)

  if (strategies.some(strategy => strategy.id !== excludedStrategyId && strategy.name.trim().toLocaleLowerCase() === trimmedName)) {
    return 'Стратегия с таким названием уже есть в этой категории'
  }

  if (strategies.some(strategy => strategy.id !== excludedStrategyId && normalizeStrategyText(strategy.content) === normalizedContent)) {
    return 'Стратегия с таким содержимым уже есть в этой категории'
  }

  return null
}

interface StrategyCardProps {
  strategy: Strategy
  isSystem: boolean
  isModified: boolean
  updateAvailable: boolean
  handleSetActive: (id: string) => void
  handleEditStrategy: (strategy: Strategy) => void
  handleClearActive: (id: string) => void
  handleDeleteStrategy: (id: string) => void
  onSystemActionClick: (strategyId: string, name: string, updateAvailable: boolean) => void
}

const StrategyCard = memo(({
  strategy,
  isSystem,
  isModified,
  updateAvailable,
  handleSetActive,
  handleEditStrategy,
  handleClearActive,
  handleDeleteStrategy,
  onSystemActionClick,
}: StrategyCardProps) => {
  const { t } = useTranslation()
  const canRestore = isSystem && (isModified || updateAvailable)

  return (
    <div
      data-strategy-id={strategy.id}
      className={cn(
        'space-y-3 rounded-lg border p-4 transition-colors',
        strategy.active
          ? 'border-success/50 bg-success/8'
          : 'border-border bg-card',
      )}
    >
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-1 min-w-0">
          <span className="font-normal">{strategy.name}</span>
          <div className="flex items-center gap-1 text-muted-foreground">
            {isSystem
              ? (
                  <InlineMarker icon={Package} label={t('category.systemBadge')} />
                )
              : (
                  <InlineMarker icon={UserRoundPlus} label={t('category.customBadge')} className="text-primary/80" />
                )}
            {strategy.active && (
              <InlineMarker icon={Check} label={t('category.activeBadge')} className="text-success animate-pulse" />
            )}
            {isModified && (
              <InlineMarker icon={FilePenLine} label={t('category.modifiedBadge')} className="text-warning" />
            )}
            {canRestore && (
              <InlineMarker
                icon={updateAvailable ? RefreshCcw : RotateCcw}
                label={updateAvailable
                  ? t('category.updateAvailable')
                  : t('category.rollbackToSystem')}
                className={updateAvailable ? 'text-primary' : 'text-destructive'}
                onClick={() => onSystemActionClick(strategy.id, strategy.name, updateAvailable)}
              />
            )}
          </div>
        </div>
        <div className="flex items-center gap-1">
          {!strategy.active && (
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  variant="outline"
                  size="icon"
                  onClick={() => handleSetActive(strategy.id)}
                  aria-label={t('category.activateAria', { name: strategy.name })}
                >
                  <Check className="size-4" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>{t('category.activateTooltip')}</TooltipContent>
            </Tooltip>
          )}
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                variant="outline"
                size="icon"
                onClick={() => handleEditStrategy(strategy)}
                aria-label={t('category.editAria', { name: strategy.name })}
              >
                <Pencil className="size-4" />
              </Button>
            </TooltipTrigger>
            <TooltipContent>{t('category.editTooltip')}</TooltipContent>
          </Tooltip>
          {strategy.active && (
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  variant="outline"
                  size="icon"
                  className="text-warning hover:text-warning"
                  onClick={() => handleClearActive(strategy.id)}
                  aria-label={t('category.deactivateAria', { name: strategy.name })}
                >
                  <BrushCleaning className="size-4" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>{t('category.deactivateTooltip')}</TooltipContent>
            </Tooltip>
          )}
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                variant="outline"
                size="icon"
                className="text-destructive hover:text-destructive"
                onClick={() => handleDeleteStrategy(strategy.id)}
                aria-label={t('category.deleteAria', { name: strategy.name })}
              >
                <Trash2 className="size-4" />
              </Button>
            </TooltipTrigger>
            <TooltipContent>{t('category.deleteTooltip')}</TooltipContent>
          </Tooltip>
        </div>
      </div>
      <pre
        className={cn(
          'overflow-x-auto rounded-md border p-3 text-xs text-muted-foreground shadow-[inset_0_1px_0_color-mix(in_oklab,var(--background)_60%,transparent)]',
          strategy.active
            ? 'border-success/30 bg-[color-mix(in_oklab,var(--success)_10%,var(--background))]'
            : 'border-border/80 bg-background/84',
        )}
      >
        {strategy.content}
      </pre>
    </div>
  )
})

type SystemActionTarget
  = | { type: 'category', title: string, description: string }
    | { type: 'strategy', strategyId: string, title: string, description: string }

export function CategoryPage() {
  const { t } = useTranslation()
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
  const [systemActionTarget, setSystemActionTarget] = useState<SystemActionTarget | null>(null)
  const newStrategyContentTextareaRef = useRef<HTMLTextAreaElement | null>(null)
  const editStrategyContentTextareaRef = useRef<HTMLTextAreaElement | null>(null)
  const scrollAreaRef = useRef<HTMLDivElement | null>(null)
  const config = useConfigStore(state => state.config)
  const builtinConfig = useConfigStore(state => state.builtinConfig)
  const loading = useConfigStore(state => state.loading)
  const load = useConfigStore(state => state.load)
  const reload = useConfigStore(state => state.reload)
  const saveNow = useConfigStore(state => state.saveNow)
  const revertTo = useConfigStore(state => state.revertTo)
  const updateCategory = useConfigStore(state => state.updateCategory)
  const restoreBuiltinCategory = useConfigStore(state => state.restoreBuiltinCategory)
  const deleteCategory = useConfigStore(state => state.deleteCategory)
  const addStrategy = useConfigStore(state => state.addStrategy)
  const updateStrategy = useConfigStore(state => state.updateStrategy)
  const restoreBuiltinStrategy = useConfigStore(state => state.restoreBuiltinStrategy)
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
  const builtinCategory = category ? getBuiltinCategory(builtinConfig, category.id) : null
  const isSystemCategoryModifiedByUser = category ? isSystemCategoryModified(category, config) : false
  const isSystemCategoryBuiltinUpdateAvailable = category ? isSystemCategoryUpdateAvailable(category, builtinCategory) : false
  const isLegacySystemCategory = !!category && isSystemCategory(category) && !builtinCategory
  const activeStrategies = category?.strategies.filter(strategy => strategy.active) ?? []
  const activeCount = activeStrategies.length
  const activeStrategiesLabel = formatActiveStrategiesLabel(activeStrategies)
  const firstActiveStrategyId = activeStrategies[0]?.id ?? null

  const getScrollViewport = () => {
    return scrollAreaRef.current?.querySelector('[data-slot="lenis-scroll-area-viewport"], [data-slot="scroll-area-viewport"]') as HTMLDivElement | null
  }

  const scrollToActiveStrategy = () => {
    if (!firstActiveStrategyId) {
      return
    }

    const viewport = getScrollViewport()
    const card = viewport?.querySelector(`[data-strategy-id="${firstActiveStrategyId}"]`)
    if (card) {
      card.scrollIntoView({
        behavior: 'smooth',
        block: 'center',
      })
    }
  }

  const handleAddStrategy = useCallback(async () => {
    if (!newStrategyName.trim() || !newStrategyContent.trim() || !categoryId) {
      return
    }

    const currentConfig = useConfigStore.getState().config
    if (!currentConfig) {
      return
    }

    const nextName = newStrategyName.trim()
    const currentCategory = currentConfig.categories.find(c => c.id === categoryId)
    const duplicateError = getStrategyDuplicateError(currentCategory?.strategies ?? [], nextName, newStrategyContent)
    if (duplicateError) {
      toast.error(duplicateError)
      return
    }

    const previousConfig = structuredClone(currentConfig)
    addStrategy(categoryId, nextName, newStrategyContent.trim())
    try {
      await saveNow()
      const latestConfig = useConfigStore.getState().config
      const latestCategory = latestConfig?.categories.find(c => c.id === categoryId)
      if (latestCategory) {
        addConfigLog(`добавлена стратегия "${nextName}" в категории "${latestCategory.name}"`)
      }
      setNewStrategyName('')
      setNewStrategyContent('')
      setNewStrategyOpen(false)
      toast.success(t('category.strategyAdded'))
    }
    catch (e) {
      revertTo(previousConfig)
      toast.error(`Ошибка сохранения стратегии: ${e instanceof Error ? e.message : String(e)}`)
    }
  }, [newStrategyName, newStrategyContent, categoryId, addStrategy, saveNow, addConfigLog, revertTo, t])

  const handleEditStrategy = useCallback((strategy: Strategy) => {
    setEditingStrategy(strategy)
    setEditingName(strategy.name)
    setEditingContent(strategy.content)
    requestAnimationFrame(() => autosizeTextarea(editStrategyContentTextareaRef.current))
  }, [])

  const handleSaveEdit = useCallback(async () => {
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
    const currentCategory = currentConfig.categories.find(c => c.id === categoryId)
    const duplicateError = getStrategyDuplicateError(currentCategory?.strategies ?? [], nextName, editingContent, editingStrategy.id)
    if (duplicateError) {
      toast.error(duplicateError)
      return
    }
    updateStrategy(categoryId, editingStrategy.id, {
      name: nextName,
      content: editingContent,
    })
    try {
      await saveNow()
      const latestConfig = useConfigStore.getState().config
      const latestCategory = latestConfig?.categories.find(c => c.id === categoryId)
      if (latestCategory) {
        addConfigLog(
          previousName !== nextName
            ? `стратегия "${previousName}" переименована в "${nextName}" в категории "${latestCategory.name}"`
            : `обновлена стратегия "${previousName}" в категории "${latestCategory.name}"`,
        )
      }
      setEditingStrategy(null)
      toast.success(t('category.strategySaved'))
    }
    catch (e) {
      revertTo(previousConfig)
      toast.error(`Ошибка сохранения стратегии: ${e instanceof Error ? e.message : String(e)}`)
    }
  }, [editingStrategy, categoryId, editingName, editingContent, updateStrategy, saveNow, addConfigLog, revertTo, t])

  const handleSetActive = useCallback(async (strategyId: string) => {
    if (!categoryId)
      return

    const currentConfig = useConfigStore.getState().config
    if (!currentConfig)
      return

    const previousConfig = structuredClone(currentConfig)

    try {
      setActiveStrategy(categoryId, strategyId)
      await saveNow()
      const latestConfig = useConfigStore.getState().config
      const latestCategory = latestConfig?.categories.find(c => c.id === categoryId)
      if (latestCategory) {
        const strategy = latestCategory.strategies.find(item => item.id === strategyId)
        if (strategy) {
          addConfigLog(`стратегия "${strategy.name}" активирована в категории "${latestCategory.name}"`)
        }
      }
      await restartIfConnected()
      notifyConfigApplied(t('category.strategyActivated'))
    }
    catch (e) {
      revertTo(previousConfig)
      toast.error(`Ошибка активации стратегии: ${e instanceof Error ? e.message : String(e)}`)
    }
  }, [categoryId, setActiveStrategy, saveNow, addConfigLog, restartIfConnected, notifyConfigApplied, revertTo, t])

  const handleClearActive = useCallback(async (strategyId: string) => {
    if (!categoryId)
      return

    const currentConfig = useConfigStore.getState().config
    if (!currentConfig)
      return

    const previousConfig = structuredClone(currentConfig)

    try {
      clearActiveStrategy(categoryId, strategyId)
      await saveNow()
      const latestConfig = useConfigStore.getState().config
      const latestCategory = latestConfig?.categories.find(c => c.id === categoryId)
      if (latestCategory) {
        const strategy = latestCategory.strategies.find(item => item.id === strategyId)
        if (strategy) {
          addConfigLog(`стратегия "${strategy.name}" деактивирована в категории "${latestCategory.name}"`)
        }
      }
      await restartIfConnected()
      notifyConfigApplied(t('category.strategyDeactivated'))
    }
    catch (e) {
      revertTo(previousConfig)
      toast.error(`Ошибка деактивации стратегии: ${e instanceof Error ? e.message : String(e)}`)
    }
  }, [categoryId, clearActiveStrategy, saveNow, addConfigLog, restartIfConnected, notifyConfigApplied, revertTo, t])

  const handleClearAllActive = useCallback(async () => {
    if (!categoryId)
      return

    const currentConfig = useConfigStore.getState().config
    if (!currentConfig)
      return

    const previousConfig = structuredClone(currentConfig)

    try {
      clearAllActiveStrategies(categoryId)
      await saveNow()
      const latestConfig = useConfigStore.getState().config
      const latestCategory = latestConfig?.categories.find(c => c.id === categoryId)
      if (latestCategory) {
        addConfigLog(`все активные стратегии отключены в категории "${latestCategory.name}"`)
      }
      await restartIfConnected()
      notifyConfigApplied(t('category.allStrategiesDeactivated'))
    }
    catch (e) {
      revertTo(previousConfig)
      toast.error(`Ошибка деактивации стратегий: ${e instanceof Error ? e.message : String(e)}`)
    }
  }, [categoryId, clearAllActiveStrategies, saveNow, addConfigLog, restartIfConnected, notifyConfigApplied, revertTo, t])

  const handleDeleteStrategy = useCallback(async (strategyId: string) => {
    if (categoryId) {
      const currentConfig = useConfigStore.getState().config
      const currentCategory = currentConfig?.categories.find(c => c.id === categoryId)
      const strategy = currentCategory?.strategies.find(s => s.id === strategyId)
      const wasActive = strategy?.active ?? false

      if (wasActive) {
        if (!currentConfig) {
          return
        }

        const previousConfig = structuredClone(currentConfig)
        deleteStrategy(categoryId, strategyId)
        try {
          await saveNow()
          const latestConfig = useConfigStore.getState().config
          const latestCategory = latestConfig?.categories.find(c => c.id === categoryId)
          if (latestCategory && strategy) {
            addConfigLog(`удалена стратегия "${strategy.name}" из категории "${latestCategory.name}"`)
          }
          toast.success(t('category.strategyDeleted'))
        }
        catch (err) {
          revertTo(previousConfig)
          console.error('Failed to save after deleting strategy:', err)
          toast.error('Ошибка сохранения после удаления стратегии')
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
        if (!currentConfig) {
          return
        }

        const previousConfig = structuredClone(currentConfig)
        deleteStrategy(categoryId, strategyId)
        try {
          await saveNow()
          const latestConfig = useConfigStore.getState().config
          const latestCategory = latestConfig?.categories.find(c => c.id === categoryId)
          if (latestCategory && strategy) {
            addConfigLog(`удалена стратегия "${strategy.name}" из категории "${latestCategory.name}"`)
          }
          toast.success(t('category.strategyDeleted'))
        }
        catch (e) {
          revertTo(previousConfig)
          toast.error(`Ошибка сохранения после удаления стратегии: ${e instanceof Error ? e.message : String(e)}`)
        }
      }
    }
  }, [categoryId, deleteStrategy, saveNow, addConfigLog, restartIfConnected, revertTo, t])

  const onSystemActionClick = useCallback((strategyId: string, name: string, updateAvailable: boolean) => {
    setSystemActionTarget({
      type: 'strategy',
      strategyId,
      title: updateAvailable
        ? t('category.updateStrategyDialogTitle')
        : t('category.restoreStrategyDialogTitle'),
      description: updateAvailable
        ? t('category.updateStrategyDialogDescription', { name })
        : t('category.restoreStrategyDialogDescription', { name }),
    })
  }, [t])

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
      toast.success(t('category.categoryDeleted'))
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
      toast.success(t('category.categoryRenamed'))
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

  const handleRestoreCategory = async () => {
    if (!category || !builtinCategory) {
      return
    }

    const currentConfig = useConfigStore.getState().config
    if (!currentConfig) {
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
      notifyConfigApplied(t('category.categoryUpdated'))
    }
    catch (error) {
      toast.error(`Категория обновлена, но не удалось применить изменения: ${error instanceof Error ? error.message : String(error)}`)
    }
    finally {
      setSystemActionTarget(null)
    }
  }

  const handleRestoreStrategy = async (strategyId: string) => {
    if (!category || !builtinCategory) {
      return
    }

    const strategy = category.strategies.find(item => item.id === strategyId)
    const builtinStrategy = getBuiltinStrategy(builtinCategory, strategyId)
    const currentConfig = useConfigStore.getState().config
    if (!strategy || !builtinStrategy || !currentConfig) {
      return
    }

    const previousConfig = structuredClone(currentConfig)
    restoreBuiltinStrategy(category.id, buildRestoredStrategy(strategy, builtinStrategy))
    try {
      await saveNow()
    }
    catch (error) {
      revertTo(previousConfig)
      toast.error(`Ошибка обновления стратегии: ${error instanceof Error ? error.message : String(error)}`)
      return
    }

    try {
      addConfigLog(`стратегия "${strategy.name}" обновлена до системного значения в категории "${category.name}"`)
      await restartIfConnected()
      notifyConfigApplied(t('category.strategyUpdated'))
    }
    catch (error) {
      toast.error(`Стратегия обновлена, но не удалось применить изменения: ${error instanceof Error ? error.message : String(error)}`)
    }
    finally {
      setSystemActionTarget(null)
    }
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <Loader2 className="size-6 animate-spin" />
      </div>
    )
  }

  if (!category) {
    return (
      <LenisScrollArea className="h-full min-h-0">
        <div className="p-6 space-y-6">
          <Link to="/strategies" className="flex items-center gap-2 text-muted-foreground hover:text-foreground">
            <ArrowLeft className="size-4" />
            {t('category.backToCategories')}
          </Link>
          <p className="text-muted-foreground">{t('category.notFound')}</p>
        </div>
      </LenisScrollArea>
    )
  }

  return (
    <div className="relative h-full min-h-0">
      <LenisScrollArea ref={scrollAreaRef} className="h-full min-h-0">
        <div className="p-6 space-y-6">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-4">
              <Link to="/strategies" className="text-muted-foreground hover:text-foreground cursor-pointer" aria-label={t('category.backToCategories')}>
                <ArrowLeft className="size-5" />
              </Link>
              <div>
                <div className="flex items-center gap-2">
                  <h1 className="text-2xl font-medium">{category.name}</h1>
                  <div className="flex items-center gap-1 text-muted-foreground">
                    {isSystemCategory(category)
                      ? (
                          <InlineMarker icon={Package} label={t('category.systemBadge')} />
                        )
                      : (
                          <InlineMarker icon={UserRoundPlus} label={t('category.customBadge')} className="text-primary/80" />
                        )}
                    {isSystemCategoryModifiedByUser && (
                      <InlineMarker icon={FilePenLine} label={t('category.modifiedBadge')} className="text-warning" />
                    )}
                    {isSystemCategory(category) && (isSystemCategoryModifiedByUser || isSystemCategoryBuiltinUpdateAvailable) && (
                      <InlineMarker
                        icon={isSystemCategoryBuiltinUpdateAvailable ? RefreshCcw : RotateCcw}
                        label={isSystemCategoryBuiltinUpdateAvailable
                          ? t('category.updateAvailable')
                          : t('category.rollbackToSystem')}
                        className={isSystemCategoryBuiltinUpdateAvailable ? 'text-primary' : 'text-destructive'}
                        onClick={() => setSystemActionTarget({
                          type: 'category',
                          title: isSystemCategoryBuiltinUpdateAvailable
                            ? t('category.updateCategoryDialogTitle')
                            : t('category.restoreCategoryDialogTitle'),
                          description: isSystemCategoryBuiltinUpdateAvailable
                            ? t('category.updateCategoryDialogDescription', { name: category.name })
                            : t('category.restoreCategoryDialogDescription', { name: category.name }),
                        })}
                      />
                    )}
                    {isLegacySystemCategory && (
                      <InlineMarker
                        icon={RotateCcw}
                        label={t('category.legacyBadge')}
                        className="text-warning"
                      />
                    )}
                    {activeCount > 0
                      ? (
                          activeStrategiesLabel && (
                            <Tooltip>
                              <TooltipTrigger asChild>
                                <button
                                  type="button"
                                  onClick={scrollToActiveStrategy}
                                  className="max-w-[14rem] cursor-pointer truncate text-xs text-success animate-pulse transition-colors hover:text-success/80"
                                >
                                  {activeStrategiesLabel}
                                </button>
                              </TooltipTrigger>
                              <TooltipContent>
                                {activeCount === 1 ? t('category.scrollToCurrent') : t('category.scrollToFirstActive')}
                              </TooltipContent>
                            </Tooltip>
                          )
                        )
                      : (
                          <Tooltip>
                            <TooltipTrigger asChild>
                              <span
                                className="inline-flex size-2 cursor-help rounded-full bg-destructive animate-pulse"
                                aria-hidden="true"
                              />
                            </TooltipTrigger>
                            <TooltipContent>{t('category.noActive')}</TooltipContent>
                          </Tooltip>
                        )}
                  </div>
                  <span className="sr-only">
                    {formatActiveStrategiesSrText(activeCount)}
                  </span>
                </div>
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
                  <Button
                    variant="outline"
                    size="icon"
                    onClick={() => {
                      void openStrategiesDirectory().catch((err: unknown) => {
                        toast.error('Не удалось открыть папку', {
                          description: err instanceof Error ? err.message : String(err),
                        })
                      })
                    }}
                    aria-label={t('category.strategiesFolderAria')}
                  >
                    <FolderOpen className="size-4" />
                  </Button>
                </TooltipTrigger>
                <TooltipContent>{t('category.strategiesFolderTooltip')}</TooltipContent>
              </Tooltip>
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button size="icon" onClick={() => setNewStrategyOpen(true)} aria-label={t('category.newStrategyAria')}>
                    <Plus className="size-4" />
                  </Button>
                </TooltipTrigger>
                <TooltipContent>{t('category.newStrategyTooltip')}</TooltipContent>
              </Tooltip>
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button variant="outline" size="icon" onClick={openRenameDialog} aria-label={t('category.renameAria', { name: category.name })}>
                    <Pencil className="size-4" />
                  </Button>
                </TooltipTrigger>
                <TooltipContent>{t('category.renameCategoryTooltip')}</TooltipContent>
              </Tooltip>
              {category.strategies.some(s => s.active) && (
                <Tooltip>
                  <TooltipTrigger asChild>
                    <Button
                      variant="outline"
                      size="icon"
                      className="text-warning hover:text-warning"
                      onClick={handleClearAllActive}
                      aria-label={t('category.deactivateAllAria')}
                    >
                      <BrushCleaning className="size-4" />
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent>{t('category.deactivateAllTooltip')}</TooltipContent>
                </Tooltip>
              )}
              <AlertDialog open={deleteDialogOpen} onOpenChange={setDeleteDialogOpen}>
                <Tooltip>
                  <TooltipTrigger asChild>
                    <AlertDialogTrigger asChild>
                      <Button
                        variant="outline"
                        size="icon"
                        className="text-destructive hover:text-destructive"
                        aria-label={t('category.deleteAria', { name: category.name })}
                      >
                        <Trash2 className="size-4" />
                      </Button>
                    </AlertDialogTrigger>
                  </TooltipTrigger>
                  <TooltipContent>{t('category.deleteCategoryTooltip')}</TooltipContent>
                </Tooltip>
                <AlertDialogContent>
                  <AlertDialogHeader>
                    <AlertDialogTitle>{t('category.deleteDialogTitle')}</AlertDialogTitle>
                    <AlertDialogDescription>
                      {t('category.deleteDialogDescription', { name: category.name })}
                    </AlertDialogDescription>
                  </AlertDialogHeader>
                  <AlertDialogFooter>
                    <AlertDialogCancel>{t('common.cancel')}</AlertDialogCancel>
                    <Button
                      onClick={async () => {
                        await handleDeleteCategory()
                      }}
                      className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
                    >
                      {t('common.delete')}
                    </Button>
                  </AlertDialogFooter>
                </AlertDialogContent>
              </AlertDialog>
            </div>
          </div>

          <div className="space-y-4">
            {category.strategies.length === 0
              ? (
                  <p className="text-sm text-muted-foreground">{t('category.noStrategies')}</p>
                )
              : (
                  category.strategies.map((strategy: Strategy) => {
                    const strategyBuiltin = getBuiltinStrategy(builtinCategory, strategy.id)
                    const isSystem = isSystemStrategy(strategy)
                    const isModified = isSystemStrategyModified(strategy)
                    const updateAvailable = isSystemStrategyUpdateAvailable(strategy, strategyBuiltin)

                    return (
                      <StrategyCard
                        key={strategy.id}
                        strategy={strategy}
                        isSystem={isSystem}
                        isModified={isModified}
                        updateAvailable={updateAvailable}
                        handleSetActive={handleSetActive}
                        handleEditStrategy={handleEditStrategy}
                        handleClearActive={handleClearActive}
                        handleDeleteStrategy={handleDeleteStrategy}
                        onSystemActionClick={onSystemActionClick}
                      />
                    )
                  })
                )}
          </div>

          <AlertDialog open={!!systemActionTarget} onOpenChange={open => !open && setSystemActionTarget(null)}>
            <AlertDialogContent>
              <AlertDialogHeader>
                <AlertDialogTitle>{systemActionTarget?.title}</AlertDialogTitle>
                <AlertDialogDescription>{systemActionTarget?.description}</AlertDialogDescription>
              </AlertDialogHeader>
              <AlertDialogFooter>
                <AlertDialogCancel>{t('common.cancel')}</AlertDialogCancel>
                <Button
                  onClick={async () => {
                    if (!systemActionTarget) {
                      return
                    }

                    if (systemActionTarget.type === 'category') {
                      await handleRestoreCategory()
                      return
                    }

                    await handleRestoreStrategy(systemActionTarget.strategyId)
                  }}
                >
                  {t('category.updateConfirm')}
                </Button>
              </AlertDialogFooter>
            </AlertDialogContent>
          </AlertDialog>

          <Dialog open={newStrategyOpen} onOpenChange={setNewStrategyOpen}>
            <DialogContent className="max-w-2xl">
              <DialogHeader>
                <DialogTitle>{t('category.newStrategyTitle')}</DialogTitle>
              </DialogHeader>
              <div className="py-4 space-y-4">
                <div className="space-y-2">
                  <Label htmlFor="strategy-name">{t('category.strategyNameLabel')}</Label>
                  <Input
                    id="strategy-name"
                    placeholder={t('category.strategyNameLabel')}
                    value={newStrategyName}
                    onChange={e => setNewStrategyName(e.target.value)}
                  />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="strategy-content">{t('category.strategyContentLabel')}</Label>
                  <EditorTextarea
                    textareaRef={newStrategyContentTextareaRef}
                    id="strategy-content"
                    placeholder="--dpi-desync=fake&#10;--dpi-desync-autottl=2"
                    value={newStrategyContent}
                    onChange={(e) => {
                      setNewStrategyContent(e.target.value)
                      autosizeTextarea(e.currentTarget)
                    }}
                    rows={10}
                  />
                  <p className="text-xs text-muted-foreground">
                    <code className="rounded bg-muted px-1 py-0.5 font-mono text-[0.72rem] text-foreground">
                      {'<LIST_MODE>'}
                    </code>
                    {' '}
                    {t('category.listModeHelp')}
                  </p>
                </div>
              </div>
              <DialogFooter>
                <Button variant="outline" onClick={() => setNewStrategyOpen(false)}>
                  {t('common.cancel')}
                </Button>
                <Button onClick={handleAddStrategy}>{t('category.createButton')}</Button>
              </DialogFooter>
            </DialogContent>
          </Dialog>

          <Dialog open={!!editingStrategy} onOpenChange={() => setEditingStrategy(null)}>
            <DialogContent className="max-w-2xl">
              <DialogHeader>
                <DialogTitle>{t('category.editStrategyTitle')}</DialogTitle>
              </DialogHeader>
              <div className="py-4 space-y-4">
                <div className="space-y-2">
                  <Label htmlFor="edit-strategy-name">{t('category.strategyNameLabel')}</Label>
                  <Input
                    id="edit-strategy-name"
                    placeholder={t('category.strategyNameLabel')}
                    value={editingName}
                    onChange={e => setEditingName(e.target.value)}
                  />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="edit-strategy-content">{t('category.strategyContentLabel')}</Label>
                  <EditorTextarea
                    textareaRef={editStrategyContentTextareaRef}
                    id="edit-strategy-content"
                    placeholder="--dpi-desync=fake&#10;--dpi-desync-autottl=2"
                    value={editingContent}
                    onChange={(e) => {
                      setEditingContent(e.target.value)
                      autosizeTextarea(e.currentTarget)
                    }}
                    rows={10}
                  />
                  <p className="text-xs text-muted-foreground">
                    <code className="rounded bg-muted px-1 py-0.5 font-mono text-[0.72rem] text-foreground">
                      {'<LIST_MODE>'}
                    </code>
                    {' '}
                    {t('category.listModeHelp')}
                  </p>
                </div>
              </div>
              <DialogFooter>
                <Button variant="outline" onClick={() => setEditingStrategy(null)}>
                  {t('common.cancel')}
                </Button>
                <Button onClick={handleSaveEdit}>{t('common.save')}</Button>
              </DialogFooter>
            </DialogContent>
          </Dialog>

          <Dialog open={renameDialogOpen} onOpenChange={setRenameDialogOpen}>
            <DialogContent>
              <DialogHeader>
                <DialogTitle>{t('category.renameCategoryTitle')}</DialogTitle>
              </DialogHeader>
              <div className="py-4">
                <div className="space-y-2">
                  <Label htmlFor="category-name">{t('category.categoryNameLabel')}</Label>
                  <Input
                    id="category-name"
                    placeholder={t('category.categoryNameLabel')}
                    value={newCategoryName}
                    onChange={e => setNewCategoryName(e.target.value)}
                    onKeyDown={e => e.key === 'Enter' && handleRenameCategory()}
                  />
                </div>
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
      <ScrollTopButton scrollAreaRef={scrollAreaRef} resetKeys={[categoryId, loading]} />
    </div>
  )
}
