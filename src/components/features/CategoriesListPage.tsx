import type {
  DragEndEvent,
} from '@dnd-kit/core'
import type { Category } from '@/lib/types'
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
import { BrushCleaning, ChevronRight, GripVertical, Loader2, Pencil, Plus, Trash2 } from 'lucide-react'
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
import { ScrollArea } from '@/components/ui/scroll-area'
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip'
import { useConfigStore } from '@/stores/config.store'
import { useConnectionStore } from '@/stores/connection.store'

interface SortableCategoryItemProps {
  category: Category
  onClearActive: (categoryId: string, e: React.MouseEvent) => void
  onRename: (category: Category) => void
  onDelete: (category: Category) => void
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

function SortableCategoryItem({ category, onClearActive, onRename, onDelete }: SortableCategoryItemProps) {
  const activeCount = category.strategies.filter(s => s.active).length

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
              <span
                className={activeCount > 0
                  ? 'inline-flex h-2 w-2 rounded-full bg-green-500 animate-pulse'
                  : 'inline-flex h-2 w-2 rounded-full bg-red-500 animate-pulse'}
                aria-hidden="true"
              />
              <span className="sr-only">
                {activeCount > 0 ? 'Есть активная стратегия' : 'Нет активной стратегии'}
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
                className="cursor-pointer"
                aria-label="Очистить стратегию"
              >
                <BrushCleaning className="w-4 h-4 text-orange-500 dark:text-orange-400" />
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
                  className="cursor-pointer border-red-500/30 bg-red-500/10 text-red-700 hover:bg-red-500/20 hover:text-red-800 dark:text-red-400 dark:hover:text-red-300"
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
  const isInitialLoadRef = useRef(true)
  const skipNextAutosaveRef = useRef(false)

  const { config, loading, load, save, addCategory, updateCategory, deleteCategory, clearAllActiveStrategies, reorderCategories } = useConfigStore()
  const { restartIfConnected, notifyConfigApplied } = useConnectionStore()

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

  useEffect(() => {
    load().finally(() => {
      isInitialLoadRef.current = false
    })
  }, [load])

  useEffect(() => {
    if (config && !isInitialLoadRef.current) {
      void save()
    }
  }, [config, save])

  const handleAddCategory = () => {
    if (newCategoryName.trim()) {
      addCategory(newCategoryName.trim())
      setNewCategoryName('')
      setNewCategoryOpen(false)
    }
  }

  const handleClearActive = async (categoryId: string, e: React.MouseEvent) => {
    e.preventDefault()
    e.stopPropagation()
    try {
      skipNextAutosaveRef.current = true
      clearAllActiveStrategies(categoryId)
      await save()
      await restartIfConnected()
      notifyConfigApplied('Стратегия деактивирована')
    }
    catch (err) {
      console.error('Failed to deactivate strategy:', err)
      notifyConfigApplied('Ошибка деактивации стратегии')
    }
    finally {
      skipNextAutosaveRef.current = false
    }
  }

  const handleDragEnd = (event: DragEndEvent) => {
    const { active, over } = event

    if (over && active.id !== over.id) {
      const oldIndex = config?.categories.findIndex(c => c.id === active.id) ?? -1
      const newIndex = config?.categories.findIndex(c => c.id === over.id) ?? -1
      if (oldIndex !== -1 && newIndex !== -1) {
        reorderCategories(oldIndex, newIndex)
      }
    }
  }

  const handleOpenRenameDialog = (category: Category) => {
    setCategoryToRename(category)
    setNewCategoryNameDraft(category.name)
    setRenameDialogOpen(true)
  }

  const handleRenameCategory = () => {
    if (categoryToRename && newCategoryNameDraft.trim()) {
      updateCategory(categoryToRename.id, newCategoryNameDraft.trim())
      setRenameDialogOpen(false)
      setCategoryToRename(null)
      setNewCategoryNameDraft('')
      toast.success('Категория переименована')
    }
  }

  const handleDeleteCategory = async (category: Category) => {
    const hadActiveStrategy = category.strategies.some(s => s.active)
    skipNextAutosaveRef.current = true
    deleteCategory(category.id)
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
        toast.error('Категория удалена, но не удалось применить изменения к активному подключению', {
          description: err instanceof Error ? err.message : String(err),
          duration: 8000,
        })
      }
    }
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
    <ScrollArea className="h-full">
      <div className="p-6 space-y-6">
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-2xl font-medium">Категории</h1>
            <p className="text-sm text-muted-foreground mt-1">
              Выберите категорию для управления стратегиями
            </p>
          </div>
          <Button onClick={() => setNewCategoryOpen(true)}>
            <Plus className="w-4 h-4 mr-2" />
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
                        onClearActive={handleClearActive}
                        onRename={handleOpenRenameDialog}
                        onDelete={handleDeleteCategory}
                      />
                    ))}
                  </SortableContext>
                </DndContext>
              )}
        </div>

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
    </ScrollArea>
  )
}
