import type { Strategy } from '@/lib/types'
import { Link, useNavigate, useParams } from '@tanstack/react-router'
import { ArrowLeft, BrushCleaning, Check, FilePenLine, Loader2, Package, Pencil, Plus, RefreshCcw, RotateCcw, Trash2, UserRoundPlus } from 'lucide-react'
import { memo, useCallback, useRef, useState } from 'react'
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
                  <InlineMarker icon={Package} label="Системная стратегия" />
                )
              : (
                  <InlineMarker icon={UserRoundPlus} label="Пользовательская стратегия" className="text-primary/80" />
                )}
            {strategy.active && (
              <InlineMarker icon={Check} label="Активная стратегия" className="text-success animate-pulse" />
            )}
            {isModified && (
              <InlineMarker icon={FilePenLine} label="Системная стратегия изменена пользователем" className="text-warning" />
            )}
            {canRestore && (
              <InlineMarker
                icon={updateAvailable ? RefreshCcw : RotateCcw}
                label={updateAvailable
                  ? 'Обновить стратегию до актуального системного значения'
                  : 'Откатить стратегию к системному значению'}
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
                  aria-label={`Активировать стратегию ${strategy.name}`}
                >
                  <Check className="size-4" />
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
                <Pencil className="size-4" />
              </Button>
            </TooltipTrigger>
            <TooltipContent>Редактировать</TooltipContent>
          </Tooltip>
          {strategy.active && (
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  variant="outline"
                  size="icon"
                  className="text-warning hover:text-warning"
                  onClick={() => handleClearActive(strategy.id)}
                  aria-label={`Деактивировать стратегию ${strategy.name}`}
                >
                  <BrushCleaning className="size-4" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>Деактивировать</TooltipContent>
            </Tooltip>
          )}
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                variant="outline"
                size="icon"
                className="text-destructive hover:text-destructive"
                onClick={() => handleDeleteStrategy(strategy.id)}
                aria-label={`Удалить стратегию ${strategy.name}`}
              >
                <Trash2 className="size-4" />
              </Button>
            </TooltipTrigger>
            <TooltipContent>Удалить</TooltipContent>
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
      toast.success('Стратегия добавлена')
    }
    catch (e) {
      revertTo(previousConfig)
      toast.error(`Ошибка сохранения стратегии: ${e instanceof Error ? e.message : String(e)}`)
    }
  }, [newStrategyName, newStrategyContent, categoryId, addStrategy, saveNow, addConfigLog, revertTo])

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
      toast.success('Стратегия сохранена')
    }
    catch (e) {
      revertTo(previousConfig)
      toast.error(`Ошибка сохранения стратегии: ${e instanceof Error ? e.message : String(e)}`)
    }
  }, [editingStrategy, categoryId, editingName, editingContent, updateStrategy, saveNow, addConfigLog, revertTo])

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
      notifyConfigApplied('Стратегия активирована')
    }
    catch (e) {
      revertTo(previousConfig)
      toast.error(`Ошибка активации стратегии: ${e instanceof Error ? e.message : String(e)}`)
    }
  }, [categoryId, setActiveStrategy, saveNow, addConfigLog, restartIfConnected, notifyConfigApplied, revertTo])

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
      notifyConfigApplied('Стратегия деактивирована')
    }
    catch (e) {
      revertTo(previousConfig)
      toast.error(`Ошибка деактивации стратегии: ${e instanceof Error ? e.message : String(e)}`)
    }
  }, [categoryId, clearActiveStrategy, saveNow, addConfigLog, restartIfConnected, notifyConfigApplied, revertTo])

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
      notifyConfigApplied('Активные стратегии отключены')
    }
    catch (e) {
      revertTo(previousConfig)
      toast.error(`Ошибка деактивации стратегий: ${e instanceof Error ? e.message : String(e)}`)
    }
  }, [categoryId, clearAllActiveStrategies, saveNow, addConfigLog, restartIfConnected, notifyConfigApplied, revertTo])

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
          toast.success('Стратегия удалена')
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
          toast.success('Стратегия удалена')
        }
        catch (e) {
          revertTo(previousConfig)
          toast.error(`Ошибка сохранения после удаления стратегии: ${e instanceof Error ? e.message : String(e)}`)
        }
      }
    }
  }, [categoryId, deleteStrategy, saveNow, addConfigLog, restartIfConnected, revertTo])

  const onSystemActionClick = useCallback((strategyId: string, name: string, updateAvailable: boolean) => {
    setSystemActionTarget({
      type: 'strategy',
      strategyId,
      title: updateAvailable
        ? 'Обновить системную стратегию?'
        : 'Откатить стратегию к системному значению?',
      description: updateAvailable
        ? `Стратегия «${name}» будет обновлена до актуальной системной версии.`
        : `Стратегия «${name}» будет возвращена к системному значению.`,
    })
  }, [])

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
      notifyConfigApplied('Категория обновлена')
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
      notifyConfigApplied('Стратегия обновлена')
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
            Назад к категориям
          </Link>
          <p className="text-muted-foreground">Категория не найдена</p>
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
              <Link to="/strategies" className="text-muted-foreground hover:text-foreground cursor-pointer" aria-label="Назад к категориям">
                <ArrowLeft className="size-5" />
              </Link>
              <div>
                <div className="flex items-center gap-2">
                  <h1 className="text-2xl font-medium">{category.name}</h1>
                  <div className="flex items-center gap-1 text-muted-foreground">
                    {isSystemCategory(category)
                      ? (
                          <InlineMarker icon={Package} label="Системная категория" />
                        )
                      : (
                          <InlineMarker icon={UserRoundPlus} label="Пользовательская категория" className="text-primary/80" />
                        )}
                    {isSystemCategoryModifiedByUser && (
                      <InlineMarker icon={FilePenLine} label="Системная категория изменена пользователем" className="text-warning" />
                    )}
                    {isSystemCategory(category) && (isSystemCategoryModifiedByUser || isSystemCategoryBuiltinUpdateAvailable) && (
                      <InlineMarker
                        icon={isSystemCategoryBuiltinUpdateAvailable ? RefreshCcw : RotateCcw}
                        label={isSystemCategoryBuiltinUpdateAvailable
                          ? 'Обновить категорию до актуального системного значения'
                          : 'Откатить категорию к системному значению'}
                        className={isSystemCategoryBuiltinUpdateAvailable ? 'text-primary' : 'text-destructive'}
                        onClick={() => setSystemActionTarget({
                          type: 'category',
                          title: isSystemCategoryBuiltinUpdateAvailable
                            ? 'Обновить системную категорию?'
                            : 'Откатить категорию к системному значению?',
                          description: isSystemCategoryBuiltinUpdateAvailable
                            ? `Категория «${category.name}» будет обновлена до актуальной системной версии. Пользовательские изменения внутри категории будут сброшены.`
                            : `Категория «${category.name}» будет возвращена к системному значению. Пользовательские изменения внутри категории будут сброшены.`,
                        })}
                      />
                    )}
                    {isLegacySystemCategory && (
                      <InlineMarker
                        icon={RotateCcw}
                        label="Системная категория из старой версии приложения"
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
                                {activeCount === 1 ? 'Прокрутить к текущей стратегии' : 'Прокрутить к первой активной стратегии'}
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
                            <TooltipContent>Нет активных стратегий</TooltipContent>
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
                  <Button size="icon" onClick={() => setNewStrategyOpen(true)} aria-label="Новая стратегия">
                    <Plus className="size-4" />
                  </Button>
                </TooltipTrigger>
                <TooltipContent>Новая стратегия</TooltipContent>
              </Tooltip>
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button variant="outline" size="icon" onClick={openRenameDialog} aria-label="Переименовать категорию">
                    <Pencil className="size-4" />
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
                      className="text-warning hover:text-warning"
                      onClick={handleClearAllActive}
                      aria-label="Деактивировать все активные стратегии"
                    >
                      <BrushCleaning className="size-4" />
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
                        className="text-destructive hover:text-destructive"
                        aria-label={`Удалить категорию ${category.name}`}
                      >
                        <Trash2 className="size-4" />
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
                <AlertDialogCancel>Отмена</AlertDialogCancel>
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
                  Обновить
                </Button>
              </AlertDialogFooter>
            </AlertDialogContent>
          </AlertDialog>

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
                    автоматически заменяется на текущий режим списков: список исключений или список заблокированных адресов.
                  </p>
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
                    автоматически заменяется на текущий режим списков: список исключений или список заблокированных адресов.
                  </p>
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
      <ScrollTopButton scrollAreaRef={scrollAreaRef} resetKeys={[categoryId, loading]} />
    </div>
  )
}
