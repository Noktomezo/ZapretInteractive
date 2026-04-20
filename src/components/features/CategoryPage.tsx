import type { Strategy } from '@/lib/types'
import { Link, useNavigate, useParams } from '@tanstack/react-router'
import { ArrowLeft, BrushCleaning, Check, FilePenLine, Loader2, Package, Pencil, Plus, RefreshCcw, RotateCcw, Trash2, UserRoundPlus } from 'lucide-react'
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
import { InlineMarker } from '@/components/ui/inline-marker'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { LenisScrollArea } from '@/components/ui/lenis-scroll-area'
import { Textarea } from '@/components/ui/textarea'
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip'
import { useMountEffect } from '@/hooks/use-mount-effect'
import { autosizeTextarea, forwardTextareaWheelToScrollArea } from '@/lib/editor-scroll'
import { buildRestoredCategory, buildRestoredStrategy, getBuiltinCategory, getBuiltinStrategy, isSystemCategory, isSystemCategoryModified, isSystemCategoryUpdateAvailable, isSystemStrategy, isSystemStrategyModified, isSystemStrategyUpdateAvailable } from '@/lib/system-config'
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
  const strategyCardRefs = useRef<Record<string, HTMLDivElement | null>>({})
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

  const scrollToActiveStrategy = () => {
    if (!firstActiveStrategyId) {
      return
    }

    strategyCardRefs.current[firstActiveStrategyId]?.scrollIntoView({
      behavior: 'smooth',
      block: 'center',
    })
  }

  const handleAddStrategy = async () => {
    if (!newStrategyName.trim() || !newStrategyContent.trim() || !categoryId) {
      return
    }

    const currentConfig = useConfigStore.getState().config
    if (!currentConfig) {
      return
    }

    const nextName = newStrategyName.trim()
    const duplicateError = getStrategyDuplicateError(category?.strategies ?? [], nextName, newStrategyContent)
    if (duplicateError) {
      toast.error(duplicateError)
      return
    }

    const previousConfig = structuredClone(currentConfig)
    addStrategy(categoryId, nextName, newStrategyContent.trim())
    try {
      await saveNow()
      if (category) {
        addConfigLog(`добавлена стратегия "${nextName}" в категории "${category.name}"`)
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
    const duplicateError = getStrategyDuplicateError(category?.strategies ?? [], nextName, editingContent, editingStrategy.id)
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
                    ref={(node) => {
                      strategyCardRefs.current[strategy.id] = node
                    }}
                    className={strategy.active
                      ? 'space-y-3 rounded-lg border border-success/40 bg-card p-4 shadow-[inset_0_0_0_1px_color-mix(in_oklab,var(--success)_18%,transparent)]'
                      : 'space-y-3 rounded-lg border bg-card p-4'}
                  >
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-3 min-w-0">
                        <span className="font-normal">{strategy.name}</span>
                        <div className="flex items-center gap-1 text-muted-foreground">
                          {strategy.active && (
                            <InlineMarker icon={Check} label="Активная стратегия" className="text-success" />
                          )}
                          {isSystemStrategy(strategy)
                            ? (
                                <InlineMarker icon={Package} label="Системная стратегия" />
                              )
                            : (
                                <InlineMarker icon={UserRoundPlus} label="Пользовательская стратегия" className="text-primary/80" />
                              )}
                          {isSystemStrategyModified(strategy) && (
                            <InlineMarker icon={FilePenLine} label="Системная стратегия изменена пользователем" className="text-warning" />
                          )}
                          {(() => {
                            const builtinStrategy = getBuiltinStrategy(builtinCategory, strategy.id)
                            const updateAvailable = isSystemStrategyUpdateAvailable(strategy, builtinStrategy)
                            const canRestore = isSystemStrategy(strategy)
                              && (isSystemStrategyModified(strategy) || updateAvailable)

                            if (!canRestore) {
                              return null
                            }

                            return (
                              <InlineMarker
                                icon={updateAvailable ? RefreshCcw : RotateCcw}
                                label={updateAvailable
                                  ? 'Обновить стратегию до актуального системного значения'
                                  : 'Откатить стратегию к системному значению'}
                                className={updateAvailable ? 'text-primary' : 'text-destructive'}
                                onClick={() => setSystemActionTarget({
                                  type: 'strategy',
                                  strategyId: strategy.id,
                                  title: updateAvailable
                                    ? 'Обновить системную стратегию?'
                                    : 'Откатить стратегию к системному значению?',
                                  description: updateAvailable
                                    ? `Стратегия «${strategy.name}» будет обновлена до актуальной системной версии.`
                                    : `Стратегия «${strategy.name}» будет возвращена к системному значению.`,
                                })}
                              />
                            )
                          })()}
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
                        {strategy.active && (
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
                    <pre className="overflow-x-auto rounded-md border border-border/80 bg-background/84 p-3 text-xs text-muted-foreground shadow-[inset_0_1px_0_color-mix(in_oklab,var(--background)_60%,transparent)]">
                      {strategy.content}
                    </pre>
                  </div>
                ))
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
                <p className="text-xs text-muted-foreground">
                  {'<LIST_MODE>'}
                  {' '}
                  автоматически заменяется на текущий режим списков: список исключений или список заблокированных адресов.
                </p>
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
                <p className="text-xs text-muted-foreground">
                  {'<LIST_MODE>'}
                  {' '}
                  автоматически заменяется на текущий режим списков: список исключений или список заблокированных адресов.
                </p>
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
