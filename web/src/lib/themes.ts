import type { ThemeInfo } from './silkprint';

export interface ThemeMeta extends ThemeInfo {
  id: string;
  label: string;
  order: number;
}

export const DEFAULT_THEME_ID = 'silkcircuit-dawn';

export const FAMILY_ORDER = [
  'silkcircuit',
  'signature',
  'developer',
  'classic',
  'nature',
  'futuristic',
  'artistic',
  'greyscale',
];

const FAMILY_LABELS: Record<string, string> = {
  artistic: 'Artistic',
  classic: 'Classic',
  developer: 'Developer',
  futuristic: 'Futuristic',
  greyscale: 'Greyscale',
  nature: 'Nature',
  signature: 'Signature',
  silkcircuit: 'SilkCircuit',
};

export const FALLBACK_THEME: ThemeMeta = {
  id: DEFAULT_THEME_ID,
  name: DEFAULT_THEME_ID,
  label: 'SilkCircuit Dawn',
  family: 'silkcircuit',
  variant: 'light',
  printSafe: true,
  description: 'Electric blue and magenta on warm cream',
  colors: { bg: '#faf8ff', fg: '#2b2540', accent: '#1565c0' },
  order: 0,
};

export function familyRank(family: string) {
  const rank = FAMILY_ORDER.indexOf(family);
  return rank === -1 ? FAMILY_ORDER.length : rank;
}

export function familyLabel(family: string) {
  return FAMILY_LABELS[family] ?? titleCase(family);
}

export function titleCase(slug: string) {
  return slug
    .split('-')
    .filter(Boolean)
    .map(part => part.charAt(0).toUpperCase() + part.slice(1))
    .join(' ');
}

export function themeLabel(id: string) {
  if (id.startsWith('silkcircuit-')) {
    return `SilkCircuit ${titleCase(id.replace('silkcircuit-', ''))}`;
  }

  return titleCase(id);
}

export function toThemeMeta(theme: ThemeInfo, order: number): ThemeMeta {
  return {
    ...theme,
    id: theme.name,
    label: themeLabel(theme.name),
    order,
  };
}

export function compareThemes(a: ThemeMeta, b: ThemeMeta) {
  const family = familyRank(a.family) - familyRank(b.family);
  if (family !== 0) return family;
  if (a.id === DEFAULT_THEME_ID) return -1;
  if (b.id === DEFAULT_THEME_ID) return 1;
  return a.order - b.order;
}

export function normalizeThemes(themes: ThemeInfo[]) {
  return themes.map((theme, index) => toThemeMeta(theme, index)).sort(compareThemes);
}

export function familyOptionsFor(themes: ThemeMeta[]) {
  const seen = new Set(themes.map(theme => theme.family));
  return [...seen]
    .sort((a, b) => familyRank(a) - familyRank(b))
    .map(family => ({ id: family, label: familyLabel(family) }));
}
