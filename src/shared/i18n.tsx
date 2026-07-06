/*
 * 核心职责：提供轻量前端 i18n。
 * 业务痛点：应用需要中/越/英切换，但不需要引入完整国际化框架。
 * 能力边界：只翻译前端静态 UI 文案，不翻译后端错误、日志和外部程序输出。
 */

import { createContext, useContext, useEffect, useMemo, useState, type ReactNode } from "react";

import enUS from "./i18n/en-US.json";
import viVN from "./i18n/vi-VN.json";

export type LanguageCode = "zh-CN" | "vi-VN" | "en-US";
export type TranslationVars = Record<string, string | number>;

const storageKey = "local-efficiency-tools-language";

export const languageOptions: Array<{ code: LanguageCode; label: string; nativeLabel: string }> = [
  { code: "zh-CN", label: "中文", nativeLabel: "中文" },
  { code: "vi-VN", label: "越南语", nativeLabel: "Tiếng Việt" },
  { code: "en-US", label: "英语", nativeLabel: "English" },
];

const dictionaries: Record<Exclude<LanguageCode, "zh-CN">, Record<string, string>> = {
  "en-US": enUS,
  "vi-VN": viVN,
};

type I18nContextValue = {
  language: LanguageCode;
  setLanguage: (language: LanguageCode) => void;
  t: (text: string, vars?: TranslationVars) => string;
};

const I18nContext = createContext<I18nContextValue | null>(null);

export function I18nProvider({ children }: { children: ReactNode }) {
  const [language, setLanguageState] = useState<LanguageCode>(() => {
    const stored = localStorage.getItem(storageKey);
    if (isLanguageCode(stored)) return stored;
    const browserLanguage = navigator.language.toLowerCase();
    if (browserLanguage.startsWith("vi")) return "vi-VN";
    if (browserLanguage.startsWith("en")) return "en-US";
    return "zh-CN";
  });

  useEffect(() => {
    document.documentElement.lang = language;
  }, [language]);

  const value = useMemo<I18nContextValue>(() => {
    const setLanguage = (nextLanguage: LanguageCode) => {
      localStorage.setItem(storageKey, nextLanguage);
      setLanguageState(nextLanguage);
    };
    return {
      language,
      setLanguage,
      t: (text, vars) => translate(language, text, vars),
    };
  }, [language]);

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useI18n() {
  const context = useContext(I18nContext);
  if (!context) {
    throw new Error("useI18n must be used inside I18nProvider");
  }
  return context;
}

function translate(language: LanguageCode, text: string, vars?: TranslationVars) {
  const template = language === "zh-CN" ? text : dictionaries[language]?.[text] ?? text;
  if (language !== "zh-CN" && template === text && import.meta.env.DEV) {
    console.warn(`[i18n] missing translation: ${text}`);
  }
  if (!vars) return template;
  return template.replace(/\{(\w+)\}/g, (match, key) => String(vars[key] ?? match));
}

function isLanguageCode(value: string | null): value is LanguageCode {
  return value === "zh-CN" || value === "vi-VN" || value === "en-US";
}
