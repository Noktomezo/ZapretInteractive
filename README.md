<div align="center">
  <img src="assets/app-thumbnail.png" alt="Zapret Interactive">

  <p>
    <strong>Настольный GUI для <a href="https://github.com/bol-van/zapret-win-bundle">zapret-win-bundle</a> с готовыми стратегиями, удобным редактированием конфигурации и встроенным управлением обновлениями.
  </p>

  <p>
    <a href="https://github.com/Noktomezo/ZapretInteractive/releases/latest"><img alt="Latest release" src="https://img.shields.io/github/v/release/Noktomezo/ZapretInteractive?style=flat&logo=github&label=%D0%B2%D0%B5%D1%80%D1%81%D0%B8%D1%8F&labelColor=1f2937&color=3b82f6&logoColor=white"></a>
    <a href="https://github.com/Noktomezo/ZapretInteractive/stargazers"><img alt="GitHub stars" src="https://img.shields.io/github/stars/Noktomezo/ZapretInteractive?style=flat&logo=github&label=%D0%B7%D0%B2%D1%91%D0%B7%D0%B4%D1%8B&labelColor=1f2937&color=f59e0b&logoColor=white"></a>
    <a href="https://github.com/Noktomezo/ZapretInteractive/releases"><img alt="GitHub all releases downloads" src="https://img.shields.io/github/downloads/Noktomezo/ZapretInteractive/total?style=flat&logo=github&label=%D1%81%D0%BA%D0%B0%D1%87%D0%B8%D0%B2%D0%B0%D0%BD%D0%B8%D1%8F&labelColor=1f2937&color=14b8a6&logoColor=white"></a>
    <a href="https://github.com/Noktomezo/ZapretInteractive/blob/main/LICENSE"><img alt="License" src="https://img.shields.io/github/license/Noktomezo/ZapretInteractive?style=flat&logo=github&label=%D0%BB%D0%B8%D1%86%D0%B5%D0%BD%D0%B7%D0%B8%D1%8F&labelColor=1f2937&color=64748b&logoColor=white"></a>
  </p>
</div>

## ✨ Фичи

- 🗂️ Управление конфигурацией прямо из GUI без ручного редактирования файлов
- 🔄 Автоматическое проверка обновлений восстановление важных файлов
- 📜 Логирование состояния подключения и внутренних событий
- ⚙️ Автозапуск, запуск в трей и автоподключение
- 🖥️ Нативный desktop UI на Tauri + React под Windows

## 📸 Скриншоты

<div align="center">
  <img width="900" height="700" alt="Zapret_Interactive_KfJFSs9bcb" src="https://github.com/user-attachments/assets/48946b34-45fc-4393-b3d2-b079bf02cee1" />
  <img width="900" height="700" alt="Zapret_Interactive_7K0CjejYUi" src="https://github.com/user-attachments/assets/989bbf63-07fa-46b6-b471-17ad9df00a07" />
</div>


## 📦 Установка

Скачать готовый установщик можно в [последних релизах](https://github.com/Noktomezo/ZapretInteractive/releases).

После установки:

- запусти приложение от имени администратора
- дождись проверки и восстановления нужных файлов
- запусти подключение
- по желанию, редактируй/удаляй/добавляй стратегии

## ⚡ Требования для запуска

- Windows 10/11 x64
- Запуск от имени администратора (для WinDivert драйвера)
- WebView2 Runtime (устанавливается автоматически)

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

## 🙏 Основа проекта

- [bol-van/zapret](https://github.com/bol-van/zapret)
- [bol-van/zapret-win-bundle](https://github.com/bol-van/zapret-win-bundle)

&nbsp;

<div align="center">
  <img src="./assets/heartbeat.svg" alt="heartbeat" width="600px">
  <p>Made with 💜. Published under <a href="LICENSE">MIT license</a>.</p>
</div>
