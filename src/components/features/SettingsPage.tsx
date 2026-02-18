import { listen } from '@tauri-apps/api/event'
import { Download, FolderOpen, Loader2, RotateCcw } from 'lucide-react'
import { useEffect, useState } from 'react'
import { toast } from 'sonner'
import { ThemeSwitcher } from '@/components/ThemeSwitcher'
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert'
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
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Switch } from '@/components/ui/switch'
import * as tauri from '@/lib/tauri'
import { useAppStore } from '@/stores/app.store'
import { useConfigStore } from '@/stores/config.store'
import { useDownloadStore } from '@/stores/download.store'

interface DownloadProgress {
  current: number
  total: number
  filename: string
  phase: 'binaries' | 'fake' | 'lists' | 'filters'
}

export function SettingsPage() {
  const [zapretDir, setZapretDir] = useState<string>('')
  const [resetDialogOpen, setResetDialogOpen] = useState(false)

  const { config, loading, load, save, setGlobalPorts, setMinimizeToTray, reset } = useConfigStore()
  const { isDownloading, progress, setDownloading, setProgress, reset: resetDownload } = useDownloadStore()
  const { binariesOk, setBinariesOk } = useAppStore()

  useEffect(() => {
    const unlistenStart = listen('download-start', () => {
      setDownloading(true)
    })

    const unlistenProgress = listen<DownloadProgress>('download-progress', (event) => {
      setProgress(event.payload)
    })

    const unlistenComplete = listen('download-complete', async () => {
      resetDownload()
      try {
        const ok = await tauri.verifyBinaries()
        setBinariesOk(ok)
      }
      catch (e) {
        toast.error(`Ошибка проверки файлов: ${e}`)
      }
    })

    const unlistenError = listen<string>('download-error', (event) => {
      console.error('Download error:', event.payload)
      resetDownload()
    })

    return () => {
      unlistenStart.then(fn => fn())
      unlistenProgress.then(fn => fn())
      unlistenComplete.then(fn => fn())
      unlistenError.then(fn => fn())
    }
  }, [])

  useEffect(() => {
    const init = async () => {
      await load()
      const dir = await tauri.getZapretDirectory()
      setZapretDir(dir)
    }
    init()
  }, [])

  useEffect(() => {
    if (config) {
      save()
    }
  }, [config])

  const handleDownloadBinaries = async () => {
    setDownloading(true)
    try {
      await tauri.downloadBinaries()
    }
    catch (e) {
      console.error(e)
      resetDownload()
      toast.error(`Ошибка загрузки файлов: ${e}`)
    }
  }

  const handleReset = async () => {
    try {
      await reset()
      setResetDialogOpen(false)
      toast.success('Настройки сброшены')
    }
    catch (e) {
      toast.error(`Ошибка сброса настроек: ${e}`)
    }
  }

  if (loading || !config) {
    return (
      <div className="flex items-center justify-center h-full">
        <Loader2 className="w-6 h-6 animate-spin" />
      </div>
    )
  }

  return (
    <div className="p-6 space-y-6">
      <div>
        <h1 className="text-2xl font-semibold">Настройки</h1>
        <p className="text-sm text-muted-foreground mt-1">
          Глобальные параметры и управление файлами
        </p>
      </div>

      <Card>
        <CardHeader>
          <CardTitle className="text-lg">Тема</CardTitle>
          <CardDescription>
            Внешний вид приложения
          </CardDescription>
        </CardHeader>
        <CardContent>
          <ThemeSwitcher />
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="text-lg">Поведение</CardTitle>
          <CardDescription>
            Настройки закрытия приложения
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="flex items-center justify-between">
            <div className="space-y-0.5">
              <Label htmlFor="minimize-to-tray">Сворачивать в трей</Label>
              <p className="text-xs text-muted-foreground">
                При закрытии окно будет скрыто в системный трей вместо завершения работы
              </p>
            </div>
            <Switch
              id="minimize-to-tray"
              checked={config.minimizeToTray ?? true}
              onCheckedChange={setMinimizeToTray}
            />
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="text-lg">Порты</CardTitle>
          <CardDescription>
            Глобальные порты для фильтрации трафика
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-2">
              <label className="text-sm font-medium">TCP порты</label>
              <Input
                value={config.global_ports.tcp}
                onChange={e =>
                  setGlobalPorts({ ...config.global_ports, tcp: e.target.value })}
                placeholder="80,443"
              />
            </div>
            <div className="space-y-2">
              <label className="text-sm font-medium">UDP порты</label>
              <Input
                value={config.global_ports.udp}
                onChange={e =>
                  setGlobalPorts({ ...config.global_ports, udp: e.target.value })}
                placeholder="1-65535"
              />
            </div>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="text-lg">Бинарные файлы</CardTitle>
          <CardDescription>
            WinDivert, winws.exe, cygwin1.dll
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex items-center gap-2">
            <FolderOpen className="w-4 h-4 text-muted-foreground" />
            <span className="text-sm font-mono">{zapretDir}</span>
          </div>

          {binariesOk === false && (
            <Alert>
              <AlertTitle>Файлы не найдены</AlertTitle>
              <AlertDescription>
                Необходимые файлы отсутствуют или повреждены
              </AlertDescription>
            </Alert>
          )}

          {binariesOk === true && (
            <Alert className="border-green-600 bg-green-600/10">
              <AlertTitle>Файлы на месте</AlertTitle>
              <AlertDescription>
                Все необходимые файлы найдены
              </AlertDescription>
            </Alert>
          )}

          {isDownloading && progress
            ? (
                <div className="space-y-2">
                  <div className="h-2 bg-muted rounded-full overflow-hidden">
                    <div
                      className="h-full bg-primary transition-all duration-300"
                      style={{
                        width: `${Math.max(0, Math.min(100, progress.total > 0 ? (progress.current / progress.total) * 100 : 0))}%`,
                      }}
                    />
                  </div>
                  <p className="text-xs text-muted-foreground">
                    {progress.current}
                    /
                    {progress.total}
                    :
                    {progress.filename}
                  </p>
                </div>
              )
            : (
                <Button
                  onClick={handleDownloadBinaries}
                  disabled={isDownloading}
                  variant={binariesOk ? 'outline' : 'default'}
                >
                  <Download className="w-4 h-4 mr-2" />
                  {binariesOk ? 'Переустановить' : 'Загрузить'}
                </Button>
              )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="text-lg">Сброс</CardTitle>
          <CardDescription>
            Возврат к настройкам по умолчанию
          </CardDescription>
        </CardHeader>
        <CardContent>
          <AlertDialog open={resetDialogOpen} onOpenChange={setResetDialogOpen}>
            <AlertDialogTrigger asChild>
              <Button variant="destructive">
                <RotateCcw className="w-4 h-4 mr-2" />
                Сбросить настройки
              </Button>
            </AlertDialogTrigger>
            <AlertDialogContent>
              <AlertDialogHeader>
                <AlertDialogTitle>Сбросить настройки?</AlertDialogTitle>
                <AlertDialogDescription>
                  Все категории, стратегии и плейсхолдеры будут удалены и заменены на значения по умолчанию. Это действие нельзя отменить.
                </AlertDialogDescription>
              </AlertDialogHeader>
              <AlertDialogFooter>
                <AlertDialogCancel>Отмена</AlertDialogCancel>
                <AlertDialogAction onClick={handleReset} className="bg-destructive text-destructive-foreground hover:bg-destructive/90">
                  Сбросить
                </AlertDialogAction>
              </AlertDialogFooter>
            </AlertDialogContent>
          </AlertDialog>
        </CardContent>
      </Card>
    </div>
  )
}
