pub fn detect_system_language() -> &'static str {
    if let Ok(lang) = std::env::var("LANG").or_else(|_| std::env::var("LC_ALL")) {
        let locale = supported_locale(&lang);
        if locale != "en" || lang.to_ascii_lowercase().starts_with("en") {
            return locale;
        }
    }

    #[cfg(target_os = "windows")]
    {
        use std::ffi::OsString;
        use std::os::windows::ffi::OsStringExt;

        unsafe extern "system" {
            fn GetUserDefaultLocaleName(lpLocaleName: *mut u16, cchLocaleName: i32) -> i32;
        }

        let mut buf = [0u16; 85];
        // SAFETY: `buf` is writable for the supplied length and remains alive for the call.
        let len = unsafe { GetUserDefaultLocaleName(buf.as_mut_ptr(), buf.len() as i32) };
        if len > 1 {
            let locale = OsString::from_wide(&buf[..(len as usize - 1)])
                .to_string_lossy()
                .into_owned();
            return supported_locale(&locale);
        }
    }

    // Fallback to English if system language is not supported
    "en"
}

pub fn supported_locale(locale: &str) -> &'static str {
    let locale = locale.to_ascii_lowercase().replace('_', "-");
    match locale.as_str() {
        value
            if value.starts_with("zh-tw")
                || value.starts_with("zh-hk")
                || value.starts_with("zh-mo")
                || value.starts_with("zh-hant") =>
        {
            "zh-TW"
        }
        value if value.starts_with("zh") => "zh-CN",
        value if value.starts_with("pt") => "pt-BR",
        value if value.starts_with("ru") => "ru",
        value if value.starts_with("fr") => "fr",
        value if value.starts_with("de") => "de",
        value if value.starts_with("es") => "es",
        value if value.starts_with("ja") => "ja",
        value if value.starts_with("ko") => "ko",
        value if value.starts_with("pl") => "pl",
        value if value.starts_with("it") => "it",
        value if value.starts_with("uk") => "uk",
        _ => "en",
    }
}

pub fn set_language(locale: &str) {
    rust_i18n::set_locale(locale);
}

pub fn t(key: &str) -> String {
    rust_i18n::t!(key).to_string()
}

#[cfg(test)]
mod tests {
    use super::supported_locale;
    use std::collections::BTreeSet;
    use std::fs;
    use std::path::Path;

    #[test]
    fn maps_supported_system_locales() {
        for (input, expected) in [
            ("de-DE", "de"),
            ("pt_BR.UTF-8", "pt-BR"),
            ("zh-Hant-HK", "zh-TW"),
            ("zh_CN", "zh-CN"),
            ("uk-UA", "uk"),
            ("nl-NL", "en"),
        ] {
            assert_eq!(supported_locale(input), expected);
        }
    }

    #[test]
    fn every_locale_has_the_same_keys() {
        let locales = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/locales");
        let russian = fs::read_to_string(locales.join("ru.yml")).expect("Russian locale exists");
        let expected = locale_keys(&russian);

        for entry in fs::read_dir(locales).expect("locale directory exists") {
            let path = entry.expect("locale entry is readable").path();
            if path.extension().is_some_and(|ext| ext == "yml") {
                let source = fs::read_to_string(&path).expect("locale is readable");
                assert_eq!(locale_keys(&source), expected, "{}", path.display());
            }
        }
    }

    fn locale_keys(source: &str) -> BTreeSet<String> {
        let mut section = "";
        let mut keys = BTreeSet::new();
        for line in source.lines().filter(|line| !line.trim().is_empty()) {
            let trimmed = line.trim();
            if !line.starts_with(' ') {
                let (name, _) = trimmed.split_once(':').expect("valid top-level locale key");
                section = name;
            } else {
                let (key, _) = trimmed.split_once(':').expect("valid locale key");
                keys.insert(format!("{section}.{key}"));
            }
        }
        keys
    }
}
