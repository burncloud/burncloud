/**
 * @license
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { createContext, useContext, useState, useEffect } from 'react';
import { SupportedLanguage, SUPPORTED_LANGUAGES, LanguageInfo, TranslationDictionary } from './types';
import { en } from './locales/en';
import { zh } from './locales/zh';
import { zhTW } from './locales/zhTW';
import { ja } from './locales/ja';

const dictionaries: Record<SupportedLanguage, TranslationDictionary> = {
  en,
  zh,
  'zh-TW': zhTW,
  ja
};

interface I18nContextType {
  language: SupportedLanguage;
  setLanguage: (lang: SupportedLanguage) => void;
  currentLangInfo: LanguageInfo;
  availableLanguages: LanguageInfo[];
  t: TranslationDictionary;
}

const I18nContext = createContext<I18nContextType | null>(null);

const STORAGE_KEY = 'burncloud_selected_language';

function getInitialLanguage(): SupportedLanguage {
  try {
    const saved = localStorage.getItem(STORAGE_KEY) as SupportedLanguage | null;
    if (saved && ['en', 'zh', 'zh-TW', 'ja'].includes(saved)) {
      return saved;
    }
    
    // Auto detect from browser
    const browserLang = navigator.language.toLowerCase();
    if (browserLang.includes('zh-tw') || browserLang.includes('zh-hk') || browserLang.includes('zh-mo') || browserLang.includes('zh-hant')) {
      return 'zh-TW';
    }
    if (browserLang.startsWith('zh')) {
      return 'zh';
    }
    if (browserLang.startsWith('ja')) {
      return 'ja';
    }
  } catch (e) {
    // Ignore storage exceptions
  }
  return 'zh'; // Default to Chinese as requested by user context, or easy switch
}

export function I18nProvider({ children }: { children: React.ReactNode }) {
  const [language, setLanguageState] = useState<SupportedLanguage>(getInitialLanguage);

  const setLanguage = (lang: SupportedLanguage) => {
    setLanguageState(lang);
    try {
      localStorage.setItem(STORAGE_KEY, lang);
      document.documentElement.lang = lang;
    } catch (e) {
      // Ignore
    }
  };

  useEffect(() => {
    try {
      document.documentElement.lang = language;
    } catch (e) {}
  }, [language]);

  const currentLangInfo =
    SUPPORTED_LANGUAGES.find((l) => l.code === language) || SUPPORTED_LANGUAGES[0];

  const t = dictionaries[language] || dictionaries.en;

  return (
    <I18nContext.Provider
      value={{
        language,
        setLanguage,
        currentLangInfo,
        availableLanguages: SUPPORTED_LANGUAGES,
        t
      }}
    >
      {children}
    </I18nContext.Provider>
  );
}

export function useI18n() {
  const context = useContext(I18nContext);
  if (!context) {
    throw new Error('useI18n must be used within an I18nProvider');
  }
  return context;
}

export function useTranslation() {
  const { t, language, setLanguage, currentLangInfo, availableLanguages } = useI18n();
  return { t, language, setLanguage, currentLangInfo, availableLanguages };
}
