import type { AppConfig, Category, Filter, Placeholder, Strategy } from './types'

export function getBuiltinCategory(builtinConfig: AppConfig | null, categoryId: string) {
  return builtinConfig?.categories.find(category => category.id === categoryId) ?? null
}

export function getBuiltinStrategy(builtinCategory: Category | null, strategyId: string) {
  return builtinCategory?.strategies.find(strategy => strategy.id === strategyId) ?? null
}

export function isSystemStrategy(strategy: Strategy) {
  return Boolean(strategy.system)
}

export function isSystemCategory(category: Category) {
  return Boolean(category.system)
}

export function getStrategyBaseName(strategy: Strategy) {
  return strategy.systemBaseName ?? strategy.name
}

export function getStrategyBaseContent(strategy: Strategy) {
  return strategy.systemBaseContent ?? strategy.content
}

export function getCategoryBaseName(category: Category) {
  return category.systemBaseName ?? category.name
}

export function isSystemStrategyModified(strategy: Strategy) {
  return isSystemStrategy(strategy) && (
    strategy.name !== getStrategyBaseName(strategy)
    || strategy.content !== getStrategyBaseContent(strategy)
  )
}

export function isSystemStrategyUpdateAvailable(strategy: Strategy, builtinStrategy: Strategy | null) {
  if (!isSystemStrategy(strategy) || !builtinStrategy) {
    return false
  }

  return getStrategyBaseName(strategy) !== builtinStrategy.name
    || getStrategyBaseContent(strategy) !== builtinStrategy.content
}

export function isSystemCategoryModified(category: Category, config: AppConfig | null) {
  if (!isSystemCategory(category)) {
    return false
  }

  const hasRemovedSystemStrategies = (config?.systemRemovedStrategyKeys ?? []).some(key => key.startsWith(`${category.id}::`))
  return category.name !== getCategoryBaseName(category)
    || category.strategies.some(strategy => !strategy.system || isSystemStrategyModified(strategy))
    || hasRemovedSystemStrategies
}

export function isSystemCategoryUpdateAvailable(category: Category, builtinCategory: Category | null) {
  if (!isSystemCategory(category) || !builtinCategory) {
    return false
  }

  if (getCategoryBaseName(category) !== builtinCategory.name) {
    return true
  }

  const currentStrategies = new Map(category.strategies.map(strategy => [strategy.id, strategy]))
  return builtinCategory.strategies.some((builtinStrategy) => {
    const strategy = currentStrategies.get(builtinStrategy.id)
    return strategy
      ? isSystemStrategyUpdateAvailable(strategy, builtinStrategy)
      : true
  })
}

export function buildRestoredStrategy(currentStrategy: Strategy | undefined, builtinStrategy: Strategy): Strategy {
  return {
    ...structuredClone(builtinStrategy),
    active: currentStrategy?.active ?? false,
  }
}

export function buildRestoredCategory(currentCategory: Category, builtinCategory: Category): Category {
  const activeByStrategyId = new Map(currentCategory.strategies.map(strategy => [strategy.id, strategy.active]))

  return {
    ...builtinCategory,
    strategies: builtinCategory.strategies.map(strategy => ({
      ...strategy,
      active: activeByStrategyId.get(strategy.id) ?? false,
    })),
  }
}

export function getBuiltinPlaceholder(builtinConfig: AppConfig | null, placeholderName: string, fallbackName?: string) {
  return builtinConfig?.placeholders.find(placeholder =>
    placeholder.name === placeholderName || (fallbackName !== undefined && placeholder.name === fallbackName),
  ) ?? null
}

export function isSystemPlaceholder(placeholder: Placeholder) {
  return Boolean(placeholder.system)
}

export function getPlaceholderBaseName(placeholder: Placeholder) {
  return placeholder.systemBaseName ?? placeholder.name
}

export function getPlaceholderBasePath(placeholder: Placeholder) {
  return placeholder.systemBasePath ?? placeholder.path
}

export function isSystemPlaceholderModified(placeholder: Placeholder) {
  return isSystemPlaceholder(placeholder)
    && (placeholder.name !== getPlaceholderBaseName(placeholder)
      || placeholder.path !== getPlaceholderBasePath(placeholder))
}

export function isSystemPlaceholderUpdateAvailable(placeholder: Placeholder, builtinPlaceholder: Placeholder | null) {
  if (!isSystemPlaceholder(placeholder) || !builtinPlaceholder) {
    return false
  }

  return getPlaceholderBaseName(placeholder) !== builtinPlaceholder.name
    || getPlaceholderBasePath(placeholder) !== builtinPlaceholder.path
}

export function buildRestoredPlaceholder(builtinPlaceholder: Placeholder): Placeholder {
  return structuredClone(builtinPlaceholder)
}

export function getBuiltinFilter(builtinConfig: AppConfig | null, filterId: string) {
  return builtinConfig?.filters.find(filter => filter.id === filterId) ?? null
}

export function isSystemFilter(filter: Filter) {
  return Boolean(filter.system)
}

export function getFilterBaseName(filter: Filter) {
  return filter.systemBaseName ?? filter.name
}

export function getFilterBaseFilename(filter: Filter) {
  return filter.systemBaseFilename ?? filter.filename
}

export function getFilterBaseContent(filter: Filter) {
  return filter.systemBaseContent ?? filter.content
}

export function isSystemFilterModified(filter: Filter) {
  return isSystemFilter(filter)
    && (filter.name !== getFilterBaseName(filter)
      || filter.filename !== getFilterBaseFilename(filter)
      || filter.content !== getFilterBaseContent(filter))
}

export function isSystemFilterUpdateAvailable(filter: Filter, builtinFilter: Filter | null) {
  if (!isSystemFilter(filter) || !builtinFilter) {
    return false
  }

  return getFilterBaseName(filter) !== builtinFilter.name
    || getFilterBaseFilename(filter) !== builtinFilter.filename
    || getFilterBaseContent(filter) !== builtinFilter.content
}

export function buildRestoredFilter(currentFilter: Filter, builtinFilter: Filter): Filter {
  return {
    ...structuredClone(builtinFilter),
    active: currentFilter.active,
  }
}
