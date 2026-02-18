import type { AppConfig, Filter, Placeholder } from './types'

export function buildFiltersCommand(filters: Filter[], filtersDir: string): string {
  const activeFilters = filters.filter(f => f.active)
  if (activeFilters.length === 0)
    return ''

  const normalizedDir = filtersDir.replace(/[/\\]+$/, '')

  return activeFilters
    .map(f => `--wf-raw-part=@${normalizedDir}\\${f.filename}`)
    .join('\n')
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

export function replacePlaceholders(content: string, placeholders: Placeholder[], homeDir?: string): string {
  let result = content
  for (const placeholder of placeholders) {
    const regex = new RegExp(`\\{\\{${placeholder.name}\\}\\}`, 'g')
    let path = placeholder.path
    if (homeDir && path.startsWith('~')) {
      path = path.replace(/^~/, homeDir)
    }
    result = result.replace(regex, path)
  }
  if (homeDir) {
    result = result.replace(/^~(?=$|\/|\\)/gm, homeDir)
  }
  return result
}

export function parseStrategyFlags(content: string): Map<string, string[]> {
  const flags = new Map<string, string[]>()
  const lines = content.split('\n')

  for (const line of lines) {
    const trimmed = line.trim()
    if (trimmed.startsWith('--')) {
      const eqIndex = trimmed.indexOf('=')
      if (eqIndex > 0) {
        const key = trimmed.slice(0, eqIndex)
        const value = trimmed.slice(eqIndex + 1)
        const existing = flags.get(key) || []
        existing.push(value)
        flags.set(key, existing)
      }
      else {
        flags.set(trimmed, [''])
      }
    }
  }

  return flags
}

export function validateStrategy(content: string): { valid: boolean, errors: string[] } {
  const errors: string[] = []
  const flags = parseStrategyFlags(content)

  const foolingValues = flags.get('--dpi-desync-fooling')
  if (foolingValues && foolingValues.length > 1) {
    errors.push('Multiple --dpi-desync-fooling values may conflict')
  }

  const desyncValues = flags.get('--dpi-desync')
  if (desyncValues && desyncValues.length > 1) {
    errors.push('Multiple --dpi-desync values in single strategy')
  }

  return {
    valid: errors.length === 0,
    errors,
  }
}

export function flagsToArgs(flags: Map<string, string[]>): string {
  const args: string[] = []

  flags.forEach((values, key) => {
    for (const value of values) {
      if (value) {
        args.push(`${key}=${value}`)
      }
      else {
        args.push(key)
      }
    }
  })

  return args.join(' ')
}
