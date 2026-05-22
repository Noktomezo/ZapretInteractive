import antfu from '@antfu/eslint-config'

export default antfu({
  ignores: [
    'src-tauri',
    'dist',
    'scripts',
    'assets',
    'AGENTS.md',
    'GEMINI.md',
    'CLAUDE.md',
  ],
})
