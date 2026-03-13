<div align="center">
  <img src="assets/app-thumbnail.png" alt="Zapret Interactive">

  <p>
    <strong>Zapret Interactive</strong> — desktop GUI для <a href="https://github.com/bol-van/zapret-win-bundle">zapret-win-bundle</a> с готовыми стратегиями, фильтрами и файлами, удобным редактированием конфигурации и встроенным управлением обновлениями.
  </p>

  <p>
    <a href="https://github.com/Noktomezo/ZapretInteractive/releases/latest"><img alt="Latest release" src="https://img.shields.io/github/v/release/Noktomezo/ZapretInteractive?style=flat&logo=github&label=%D0%B2%D0%B5%D1%80%D1%81%D0%B8%D1%8F&labelColor=1f2937&color=3b82f6&logoColor=white"></a>
    <a href="https://github.com/Noktomezo/ZapretInteractive/stargazers"><img alt="GitHub stars" src="https://img.shields.io/github/stars/Noktomezo/ZapretInteractive?style=flat&logo=github&label=%D0%B7%D0%B2%D1%91%D0%B7%D0%B4%D1%8B&labelColor=1f2937&color=f59e0b&logoColor=white"></a>
    <a href="https://github.com/Noktomezo/ZapretInteractive/releases"><img alt="GitHub all releases downloads" src="https://img.shields.io/github/downloads/Noktomezo/ZapretInteractive/total?style=flat&logo=github&label=%D1%81%D0%BA%D0%B0%D1%87%D0%B8%D0%B2%D0%B0%D0%BD%D0%B8%D1%8F&labelColor=1f2937&color=14b8a6&logoColor=white"></a>
    <a href="https://github.com/Noktomezo/ZapretInteractive/blob/main/LICENSE"><img alt="License" src="https://img.shields.io/github/license/Noktomezo/ZapretInteractive?style=flat&logo=github&label=%D0%BB%D0%B8%D1%86%D0%B5%D0%BD%D0%B7%D0%B8%D1%8F&labelColor=1f2937&color=64748b&logoColor=white"></a>
  </p>

  <p>
    Windows-only · Tauri + React · Стратегии, фильтры, плейсхолдеры, автообновления и восстановление файлов
  </p>

</div>

## ✨ Что умеет

- 🗂️ Управление категориями и стратегиями прямо из GUI без ручного редактирования файлов
- 🧩 Редактирование фильтров и плейсхолдеров в одном интерфейсе
- 🔄 Автоматическое восстановление важных файлов в `~/.zapret`
- ⬆️ Проверка обновлений приложения, `winws` и связанных файлов
- 📜 Отдельная страница логов для состояния подключения и внутренних событий
- ⚙️ Автозапуск, запуск в трей и автоподключение
- 🖥️ Нативный desktop UI на Tauri + React под Windows

## 📸 Скриншот

<div align="center">
  <img src="assets/app-screenshot.png" alt="Screenshot" width="100%">
</div>

## 📦 Установка

Скачать готовый установщик можно в [последних релизах](https://github.com/Noktomezo/ZapretInteractive/releases).

После установки:

- запусти приложение от имени администратора
- дождись проверки и восстановления нужных файлов
- выбери стратегию и включи подключение

## ⚡ Требования для запуска

- Windows 10/11 x64
- Запуск от имени администратора (для WinDivert драйвера)
- WebView2 Runtime (устанавливается автоматически)

## 🧠 Что внутри

- стратегии для разных сервисов и сценариев
- управление списками, фильтрами и плейсхолдерами
- самовосстановление `config.json`, `fake/`, `lists/` и других критичных файлов
- ручные и автоматические сценарии обновления
- NSIS installer и автообновление самого приложения

## 🛠️ Разработка

### Зависимости

- [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) с выбранным **Desktop development with C++** (MSVC + Windows SDK)
- [Bun](https://bun.com/)
- [Just](https://github.com/casey/just)

### Основные команды

```bash
# Установка зависимостей
just bootstrap

# Запуск в режиме разработки
just dev

# Сборка релиза
just build
```

### Локальная сборка installer без signing key

```bash
just build-local
```

Этот путь отключает локальную генерацию updater artifacts и нужен для проверки installer UI без приватного ключа.

## 🙏 Основа проекта

- [bol-van/zapret](https://github.com/bol-van/zapret)
- [bol-van/zapret-win-bundle](https://github.com/bol-van/zapret-win-bundle)

&nbsp;

<div align="center">
  <img src="./assets/heartbeat.svg" alt="heartbeat" width="600px">
  <p>Made with 💜. Published under <a href="LICENSE">MIT license</a>.</p>
</div>
