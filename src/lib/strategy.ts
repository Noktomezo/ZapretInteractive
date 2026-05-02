import type { AppConfig, Filter } from './types'

export function buildFiltersCommand(filters: Filter[], filtersDir: string): string {
  const activeFilters = filters.filter(f => f.active)
  if (activeFilters.length === 0)
    return ''

  const normalizedDir = filtersDir.replace(/[/\\]+$/, '')

  return activeFilters
    .map(f => `--wf-raw-part=@${normalizedDir}\\${f.filename}`)
    .join('\n')
}

export function buildFiltersCommandArray(filters: Filter[], filtersDir: string): string[] {
  const activeFilters = filters.filter(f => f.active)
  if (activeFilters.length === 0)
    return []

  const normalizedDir = filtersDir.replace(/[/\\]+$/, '')

  return activeFilters.map(f => `--wf-raw-part=@${normalizedDir}\\${f.filename}`)
}

export function buildStrategyCommand(config: AppConfig): string {
  const activeStrategies: string[] = []

  const listModeArg = (config.listMode ?? 'exclude') === 'exclude'
    ? '--hostlist-exclude={{HOSTS_USER_EXCLUDE}}'
    : '--ipset={{IP_USER}}'

  for (const category of config.categories) {
    for (const strategy of category.strategies) {
      if (strategy.active) {
        const content = strategy.content.replace(/<LIST_MODE>/g, listModeArg)
        activeStrategies.push(content)
      }
    }
  }

  return activeStrategies.join('\n--new\n')
}
