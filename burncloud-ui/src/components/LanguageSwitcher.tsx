/**
 * @license
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useState, useRef, useEffect } from 'react';
import { Globe, Check, ChevronDown } from 'lucide-react';
import { useI18n } from '@/i18n/I18nContext';
import { SupportedLanguage } from '@/i18n/types';
import { cn } from '@/lib/utils';

interface LanguageSwitcherProps {
  variant?: 'navbar' | 'compact' | 'footer' | 'pill';
  className?: string;
}

export function LanguageSwitcher({ variant = 'navbar', className }: LanguageSwitcherProps) {
  const { language, setLanguage, currentLangInfo, availableLanguages, t } = useI18n();
  const [isOpen, setIsOpen] = useState(false);
  const dropdownRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    function handleClickOutside(event: MouseEvent) {
      if (dropdownRef.current && !dropdownRef.current.contains(event.target as Node)) {
        setIsOpen(false);
      }
    }
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  const handleSelect = (code: SupportedLanguage) => {
    setLanguage(code);
    setIsOpen(false);
  };

  if (variant === 'pill') {
    return (
      <div className={cn("inline-flex p-0.5 rounded-xl bg-gray-100/80 border border-gray-200/80", className)}>
        {availableLanguages.map((lang) => (
          <button
            key={lang.code}
            onClick={() => setLanguage(lang.code)}
            className={cn(
              "px-2.5 py-1 text-xs rounded-lg font-medium transition-all flex items-center gap-1.5",
              language === lang.code
                ? "bg-white text-gray-900 shadow-xs font-semibold"
                : "text-gray-600 hover:text-gray-950"
            )}
          >
            <span>{lang.flag}</span>
            <span>{lang.short}</span>
          </button>
        ))}
      </div>
    );
  }

  return (
    <div className={cn("relative inline-block text-left", className)} ref={dropdownRef}>
      <button
        type="button"
        id="language-switcher-button"
        onClick={() => setIsOpen(!isOpen)}
        className={cn(
          "flex items-center gap-2 rounded-xl transition-all focus:outline-none",
          variant === 'navbar' &&
            "h-8 px-2.5 bg-gray-50 hover:bg-gray-100 border border-gray-200/80 text-xs font-medium text-gray-700 shadow-2xs",
          variant === 'compact' &&
            "p-1.5 hover:bg-gray-100 text-gray-600 rounded-lg text-xs",
          variant === 'footer' &&
            "h-8 px-3 bg-white hover:bg-gray-50 border border-gray-200 text-xs text-gray-700 rounded-lg shadow-2xs"
        )}
        aria-expanded={isOpen}
        aria-haspopup="true"
        title={t.common.selectLanguage}
      >
        <Globe className="w-3.5 h-3.5 text-gray-500 flex-shrink-0" />
        <span className="flex items-center gap-1.5">
          <span className="text-sm leading-none">{currentLangInfo.flag}</span>
          <span className="font-medium text-xs text-gray-800">{currentLangInfo.nativeName}</span>
        </span>
        <ChevronDown
          className={cn(
            "w-3.5 h-3.5 text-gray-400 transition-transform duration-200 flex-shrink-0",
            isOpen && "rotate-180 text-gray-700"
          )}
        />
      </button>

      {isOpen && (
        <div
          id="language-dropdown-menu"
          className="absolute right-0 mt-1.5 w-44 rounded-xl bg-white p-1.5 shadow-xl ring-1 ring-black/5 border border-gray-200/90 z-50 animate-in fade-in-0 zoom-in-95 duration-100"
        >
          <div className="px-2.5 py-1 text-[10px] font-mono font-bold uppercase tracking-wider text-gray-400 border-b border-gray-100 mb-1">
            {t.common.selectLanguage}
          </div>
          <div className="space-y-0.5">
            {availableLanguages.map((lang) => {
              const isSelected = language === lang.code;
              return (
                <button
                  key={lang.code}
                  id={`lang-opt-${lang.code}`}
                  onClick={() => handleSelect(lang.code)}
                  className={cn(
                    "w-full flex items-center justify-between px-2.5 py-1.5 rounded-lg text-xs font-medium transition-colors text-left group",
                    isSelected
                      ? "bg-gray-900 text-white font-semibold shadow-xs"
                      : "text-gray-700 hover:bg-gray-100 hover:text-gray-900"
                  )}
                >
                  <div className="flex items-center gap-2">
                    <span className="text-base leading-none">{lang.flag}</span>
                    <span className="flex flex-col">
                      <span className={isSelected ? "text-white" : "text-gray-900"}>
                        {lang.nativeName}
                      </span>
                      <span
                        className={cn(
                          "text-[10px]",
                          isSelected ? "text-gray-300" : "text-gray-400 group-hover:text-gray-500"
                        )}
                      >
                        {lang.name}
                      </span>
                    </span>
                  </div>
                  {isSelected && <Check className="w-3.5 h-3.5 text-white flex-shrink-0" />}
                </button>
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
}
