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
import { ChevronRight, Eraser, GripVertical, Loader2, Plus } from 'lucide-react'
import { useEffect, useRef, useState } from 'react'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip'
import { useConfigStore } from '@/stores/config.store'

interface SortableCategoryItemProps {
  category: Category
  onClearActive: (categoryId: string, e: React.MouseEvent) => void
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

function SortableCategoryItem({ category, onClearActive }: SortableCategoryItemProps) {
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
        className="flex min-w-0 flex-1 cursor-pointer items-center justify-between rounded-md"
      >
        <div className="flex min-w-0 items-center gap-3">
          <div className="min-w-0 space-y-1">
            <div className="flex items-center gap-3">
              <span className="truncate text-sm font-normal">{category.name}</span>
              {activeCount > 0 && (
                <span className="rounded bg-green-600 px-2 py-0.5 text-xs text-white">
                  активна
                </span>
              )}
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
      {activeCount > 0 && (
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="icon"
              onClick={e => onClearActive(category.id, e)}
              className="cursor-pointer hover:bg-red-500/10 [&:hover_svg]:text-red-500 transition-colors duration-200"
            >
              <Eraser className="w-4 h-4 text-muted-foreground" />
            </Button>
          </TooltipTrigger>
          <TooltipContent>Деактивировать текущую стратегию</TooltipContent>
        </Tooltip>
      )}
    </div>
  )
}

export function CategoriesListPage() {
  const [newCategoryOpen, setNewCategoryOpen] = useState(false)
  const [newCategoryName, setNewCategoryName] = useState('')
  const isInitialLoadRef = useRef(true)

  const { config, loading, load, save, addCategory, clearAllActiveStrategies, reorderCategories } = useConfigStore()

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
  }, [])

  useEffect(() => {
    if (config && !isInitialLoadRef.current) {
      save()
    }
  }, [config])

  const handleAddCategory = () => {
    if (newCategoryName.trim()) {
      addCategory(newCategoryName.trim())
      setNewCategoryName('')
      setNewCategoryOpen(false)
    }
  }

  const handleClearActive = (categoryId: string, e: React.MouseEvent) => {
    e.preventDefault()
    e.stopPropagation()
    clearAllActiveStrategies(categoryId)
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
            <Input
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
    </div>
  )
}
