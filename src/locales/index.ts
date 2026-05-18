import en from './en';
import zhCN from './zh-CN';

export type LocaleMessages = typeof zhCN;
export type LocaleKey = FlattenKeys<LocaleMessages>;

type FlattenKeys<T, Prefix extends string = ''> = T extends string
  ? Prefix
  : T extends object
    ? {
        [K in keyof T & string]: FlattenKeys<T[K], Prefix extends '' ? K : `${Prefix}.${K}`>;
      }[keyof T & string]
    : never;

export const localeMessages = { 'zh-CN': zhCN, en } as const;
export const supportedLocales = ['zh-CN', 'en'] as const;
export type SupportedLocale = (typeof supportedLocales)[number];
