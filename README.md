<div align="center">
  <img src="assets/app-thumbnail.png" alt="Zapret Interactive">

  <p>
    <strong>Настольный интерфейс для <a href="https://github.com/bol-van/zapret-win-bundle">zapret-win-bundle</a> с готовыми стратегиями, модулями и автоматическим восстановлением управляемых файлов.</strong>
  </p>

  <p>
    <a href="https://github.com/Noktomezo/ZapretInteractive/releases/latest"><img alt="Latest release" src="https://img.shields.io/github/v/release/Noktomezo/ZapretInteractive?style=flat&logo=github&label=%D0%B2%D0%B5%D1%80%D1%81%D0%B8%D1%8F&labelColor=1f2937&color=3b82f6&logoColor=white"></a>
    <a href="https://github.com/Noktomezo/ZapretInteractive/stargazers"><img alt="GitHub stars" src="https://img.shields.io/github/stars/Noktomezo/ZapretInteractive?style=flat&logo=github&label=%D0%B7%D0%B2%D1%91%D0%B7%D0%B4%D1%8B&labelColor=1f2937&color=f59e0b&logoColor=white"></a>
    <a href="https://github.com/Noktomezo/ZapretInteractive/releases"><img alt="GitHub downloads" src="https://img.shields.io/github/downloads/Noktomezo/ZapretInteractive/total?style=flat&logo=github&label=%D1%81%D0%BA%D0%B0%D1%87%D0%B8%D0%B2%D0%B0%D0%BD%D0%B8%D1%8F&labelColor=1f2937&color=14b8a6&logoColor=white"></a>
  </p>
</div>

## ✨ Возможности

- Подключение и отключение zapret из одного окна.
- Категории стратегий с выбором, редактированием и drag-and-drop сортировкой.
- Модули Smart DNS и Telegram WebSocket Proxy.
- Управление фильтрами, плейсхолдерами и списками.
- Проверка обновлений приложения и модулей, автоматическое восстановление управляемых файлов.
- Автозапуск, системный трей, автоподключение и Discord Rich Presence.
- Нативный интерфейс на GPUI без WebView.

## 🤔 Установка

1. Скачайте `Zapret.Interactive_X.X.X_x64-installer.exe` из [последнего релиза](https://github.com/Noktomezo/ZapretInteractive/releases/latest).
2. Запустите установщик.
3. Откройте Zapret Interactive от имени администратора.

## 😭 Требования

- Windows 10/11 x64.
- Права администратора.

## 🛠️ Разработка

Нужны Rust stable, Visual Studio Build Tools с компонентами MSVC и Windows SDK, [Just](https://github.com/casey/just) и [watchexec](https://github.com/watchexec/watchexec).

```powershell
just dev       # запуск с автоматической пересборкой
just check     # проверка компиляции
just test      # тесты
just strict    # check, test, clippy и rustfmt
just build     # релизная сборка и установщик
```

Обновление и проверка содержимого `thirdparty`:

```powershell
just update-thirdparty
just verify-thirdparty
```

## 🙏 Благодарности

- [bol-van/zapret](https://github.com/bol-van/zapret)
- [bol-van/zapret-win-bundle](https://github.com/bol-van/zapret-win-bundle)
- [StressOzz/Zapret-Manager](https://github.com/StressOzz/Zapret-Manager)
- [DNSCrypt/dnscrypt-proxy](https://github.com/DNSCrypt/dnscrypt-proxy)
- [valnesfjord/tg-ws-proxy-rs](https://github.com/valnesfjord/tg-ws-proxy-rs)
- [kepano/flexoki](https://github.com/kepano/flexoki)
