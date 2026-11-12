// UI strings. English only for now; every string still goes through t() so translations can be
// added as new catalogs with the same keys. Placeholders look like {name}. A plural entry lists
// Intl.PluralRules categories and is picked by the {count} parameter.
import { en, type Key, type PluralForms } from "./en";

export type { Key };

type Params = Record<string, string | number>;

const rules = new Intl.PluralRules("en");

function fill(text: string, params?: Params): string {
  if (!params) return text;
  return text.replace(/\{(\w+)\}/g, (m, k) => (k in params ? String(params[k]) : m));
}

/** Translate `key`. Plural entries use `params.count`. */
export function t(key: Key, params?: Params): string {
  const entry = en[key] as string | PluralForms;
  const text = typeof entry === "string" ? entry : (entry[rules.select(Number(params?.count ?? 0))] ?? entry.other);
  return fill(text, params);
}
