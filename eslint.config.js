import antfu from '@antfu/eslint-config'

export default antfu({
  ignores: ['src-tauri', 'node_modules', 'dist', 'scripts', 'assets'],
})
