export type DictEntry = { pos: string; terms: string[] };

export type Translation = {
  text: string;
  from: string;
  to: string;
  engine: string;
  phonetic: string | null;
  dict: DictEntry[];
  definitions: DictEntry[];
  note: string | null;
  context?: TranslationContext | null;
};

export type TranslationContext = {
  sentence: string;
  translation: string;
  meaning: string | null;
  engine: string;
};

export type PopupState =
  | { mode: "hidden" }
  | { mode: "icon" }
  | { mode: "loading"; source: string }
  | { mode: "result"; source: string; result: Translation; partial?: boolean }
  | { mode: "error"; source: string; message: string };

export type Profile = { app: string; engine: string; primary_lang: string };

export type Settings = {
  hotkey: string;
  ocr_hotkey: string;
  auto_trigger: boolean;
  hover_translate: boolean;
  engine: string;
  fallback_offline: boolean;
  primary_lang: string;
  secondary_lang: string;
  deepl_key: string;
  anthropic_key: string;
  excluded_apps: string[];
  save_history: boolean;
  usage_stats: boolean;
  theme: "system" | "light" | "dark";
  english_definitions: boolean;
  oxford_app_id: string;
  oxford_app_key: string;
  window_hotkey: string;
  live_hotkey: string;
  clipboard_hotkey: string;
  instant_translate: boolean;
  hover_sentence: boolean;
  listen_hide_captions: boolean;
  explain_in_context: boolean;
  check_updates: boolean;
  popup_font_size: number;
  popup_opacity: number;
  openai_base: string;
  openai_key: string;
  openai_model: string;
  profiles: Profile[];
};

export type UpdateInfo = {
  current: string;
  latest: string;
  available: boolean;
  notes: string;
  published: string;
  page: string;
};

export type HistoryEntry = {
  at: number;
  source: string;
  text: string;
  from: string;
  to: string;
  engine: string;
};

export const LANGUAGES: [string, string][] = [
  ["vi", "Tiếng Việt"],
  ["en", "Tiếng Anh"],
  ["ja", "Tiếng Nhật"],
  ["ko", "Tiếng Hàn"],
  ["zh", "Tiếng Trung"],
  ["fr", "Tiếng Pháp"],
  ["de", "Tiếng Đức"],
  ["es", "Tiếng Tây Ban Nha"],
  ["pt", "Tiếng Bồ Đào Nha"],
  ["it", "Tiếng Ý"],
  ["ru", "Tiếng Nga"],
  ["th", "Tiếng Thái"],
  ["id", "Tiếng Indonesia"],
  ["ms", "Tiếng Mã Lai"],
  ["hi", "Tiếng Hindi"],
  ["ar", "Tiếng Ả Rập"],
  ["tr", "Tiếng Thổ Nhĩ Kỳ"],
  ["nl", "Tiếng Hà Lan"],
  ["pl", "Tiếng Ba Lan"],
  ["uk", "Tiếng Ukraina"],
];

export const ENGINE_NAMES: Record<string, string> = {
  google: "Google",
  deepl: "DeepL",
  claude: "Claude",
  offline: "Offline",
  openai: "AI (OpenAI API)",
};

export function langName(code: string): string {
  const base = code.toLowerCase().split("-")[0];
  return LANGUAGES.find(([c]) => c === base)?.[1] ?? code.toUpperCase();
}

export type Card = {
  at: number;
  front: string;
  back: string;
  detail: string;
  from: string;
  to: string;
};
