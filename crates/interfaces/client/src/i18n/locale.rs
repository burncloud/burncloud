//! Locale selection and metadata for the console UI.

use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum Locale {
    Zh,
    En,
    ZhTw,
    Ja,
}

impl Default for Locale {
    fn default() -> Self {
        Self::Zh
    }
}

impl Locale {
    // Keep the menu order aligned with the reference console: English first,
    // followed by Simplified Chinese, Traditional Chinese, and Japanese.
    pub const ALL: [Self; 4] = [Self::En, Self::Zh, Self::ZhTw, Self::Ja];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Zh => "中文",
            Self::En => "English",
            Self::ZhTw => "繁體中文",
            Self::Ja => "日本語",
        }
    }

    pub const fn flag(self) -> &'static str {
        match self {
            Self::Zh => "🇨🇳",
            Self::En => "🇺🇸",
            Self::ZhTw => "🇭🇰",
            Self::Ja => "🇯🇵",
        }
    }

    pub const fn native_name(self) -> &'static str {
        match self {
            Self::Zh => "简体中文",
            Self::En => "English",
            Self::ZhTw => "繁體中文",
            Self::Ja => "日本語",
        }
    }

    pub const fn english_name(self) -> &'static str {
        match self {
            Self::Zh => "Simplified Chinese",
            Self::En => "English",
            Self::ZhTw => "Traditional Chinese",
            Self::Ja => "Japanese",
        }
    }

    pub const fn code(self) -> &'static str {
        match self {
            Self::Zh => "zh-CN",
            Self::En => "en-US",
            Self::ZhTw => "zh-TW",
            Self::Ja => "ja-JP",
        }
    }

    pub const fn storage_code(self) -> &'static str {
        match self {
            Self::Zh => "zh",
            Self::En => "en",
            Self::ZhTw => "zh-TW",
            Self::Ja => "ja",
        }
    }

    pub const fn browser_code(self) -> &'static str {
        self.code()
    }

    pub fn from_code(value: &str) -> Self {
        match value {
            "en" | "en-US" | "en-GB" => Self::En,
            "zh-TW" | "zh-HK" | "zh-Hant" => Self::ZhTw,
            "ja" | "ja-JP" => Self::Ja,
            _ => Self::Zh,
        }
    }
}

impl fmt::Display for Locale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}
