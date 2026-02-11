'use client';

import { useCallback, useEffect, useMemo, useRef, useState } from 'react';

// ── Static theme metadata ────────────────────────────────────────
// All 40 built-in themes with visual colors extracted from TOML sources.
// This data is static so the selector works before WASM loads.

interface ThemeMeta {
  id: string;
  name: string;
  family: string;
  variant: 'light' | 'dark';
  printSafe: boolean;
  colors: { bg: string; fg: string; accent: string };
}

const FAMILIES = [
  { id: 'silkcircuit', label: 'SilkCircuit' },
  { id: 'signature', label: 'Signature' },
  { id: 'developer', label: 'Developer' },
  { id: 'classic', label: 'Classic' },
  { id: 'nature', label: 'Nature' },
  { id: 'futuristic', label: 'Futuristic' },
  { id: 'artistic', label: 'Artistic' },
  { id: 'greyscale', label: 'Greyscale' },
] as const;

const THEMES: ThemeMeta[] = [
  // SilkCircuit — Dawn first (default theme)
  {
    id: 'silkcircuit-dawn',
    name: 'SilkCircuit Dawn',
    family: 'silkcircuit',
    variant: 'light',
    printSafe: true,
    colors: { bg: '#faf8ff', fg: '#2b2540', accent: '#1565c0' },
  },
  {
    id: 'silkcircuit-neon',
    name: 'SilkCircuit Neon',
    family: 'silkcircuit',
    variant: 'dark',
    printSafe: false,
    colors: { bg: '#12101a', fg: '#f8f8f2', accent: '#80ffea' },
  },
  {
    id: 'silkcircuit-vibrant',
    name: 'SilkCircuit Vibrant',
    family: 'silkcircuit',
    variant: 'dark',
    printSafe: false,
    colors: { bg: '#0f0c1a', fg: '#f8f8f2', accent: '#00ffcc' },
  },
  {
    id: 'silkcircuit-soft',
    name: 'SilkCircuit Soft',
    family: 'silkcircuit',
    variant: 'dark',
    printSafe: false,
    colors: { bg: '#1a1626', fg: '#f8f8f2', accent: '#99ffee' },
  },
  {
    id: 'silkcircuit-glow',
    name: 'SilkCircuit Glow',
    family: 'silkcircuit',
    variant: 'dark',
    printSafe: false,
    colors: { bg: '#0a0816', fg: '#ffffff', accent: '#00ffff' },
  },
  // Signature
  {
    id: 'silk-light',
    name: 'Silk Light',
    family: 'signature',
    variant: 'light',
    printSafe: true,
    colors: { bg: '#ffffff', fg: '#1a1a2e', accent: '#4a5dbd' },
  },
  {
    id: 'silk-dark',
    name: 'Silk Dark',
    family: 'signature',
    variant: 'dark',
    printSafe: false,
    colors: { bg: '#12121e', fg: '#e0e0f0', accent: '#7b93db' },
  },
  {
    id: 'manuscript',
    name: 'Manuscript',
    family: 'signature',
    variant: 'light',
    printSafe: true,
    colors: { bg: '#F5EDE4', fg: '#2C2419', accent: '#4A3728' },
  },
  {
    id: 'monochrome',
    name: 'Monochrome',
    family: 'signature',
    variant: 'light',
    printSafe: true,
    colors: { bg: '#FFFFFF', fg: '#000000', accent: '#000000' },
  },
  // Developer
  {
    id: 'nord',
    name: 'Nord',
    family: 'developer',
    variant: 'dark',
    printSafe: false,
    colors: { bg: '#2E3440', fg: '#D8DEE9', accent: '#88C0D0' },
  },
  {
    id: 'dracula',
    name: 'Dracula',
    family: 'developer',
    variant: 'dark',
    printSafe: false,
    colors: { bg: '#282A36', fg: '#F8F8F2', accent: '#BD93F9' },
  },
  {
    id: 'solarized-light',
    name: 'Solarized Light',
    family: 'developer',
    variant: 'light',
    printSafe: true,
    colors: { bg: '#FDF6E3', fg: '#5B6F77', accent: '#073642' },
  },
  {
    id: 'solarized-dark',
    name: 'Solarized Dark',
    family: 'developer',
    variant: 'dark',
    printSafe: false,
    colors: { bg: '#002B36', fg: '#839496', accent: '#93A1A1' },
  },
  {
    id: 'catppuccin-latte',
    name: 'Catppuccin Latte',
    family: 'developer',
    variant: 'light',
    printSafe: true,
    colors: { bg: '#EFF1F5', fg: '#4C4F69', accent: '#8839EF' },
  },
  {
    id: 'catppuccin-mocha',
    name: 'Catppuccin Mocha',
    family: 'developer',
    variant: 'dark',
    printSafe: false,
    colors: { bg: '#1E1E2E', fg: '#CDD6F4', accent: '#CBA6F7' },
  },
  {
    id: 'gruvbox-light',
    name: 'Gruvbox Light',
    family: 'developer',
    variant: 'light',
    printSafe: true,
    colors: { bg: '#FBF1C7', fg: '#3C3836', accent: '#B57614' },
  },
  {
    id: 'gruvbox-dark',
    name: 'Gruvbox Dark',
    family: 'developer',
    variant: 'dark',
    printSafe: false,
    colors: { bg: '#282828', fg: '#EBDBB2', accent: '#FABD2F' },
  },
  {
    id: 'tokyo-night',
    name: 'Tokyo Night',
    family: 'developer',
    variant: 'dark',
    printSafe: false,
    colors: { bg: '#1A1B26', fg: '#A9B1D6', accent: '#7AA2F7' },
  },
  {
    id: 'rose-pine',
    name: 'Rose Pine',
    family: 'developer',
    variant: 'dark',
    printSafe: false,
    colors: { bg: '#191724', fg: '#E0DEF4', accent: '#C4A7E7' },
  },
  // Classic
  {
    id: 'academic',
    name: 'Academic',
    family: 'classic',
    variant: 'light',
    printSafe: true,
    colors: { bg: '#FAFAF7', fg: '#1A1A24', accent: '#2B4D8C' },
  },
  {
    id: 'typewriter',
    name: 'Typewriter',
    family: 'classic',
    variant: 'light',
    printSafe: true,
    colors: { bg: '#F2EDE4', fg: '#1C1915', accent: '#6B4F3A' },
  },
  {
    id: 'newspaper',
    name: 'Newspaper',
    family: 'classic',
    variant: 'light',
    printSafe: true,
    colors: { bg: '#F0EDE5', fg: '#1A1A1A', accent: '#8C1A1A' },
  },
  {
    id: 'parchment',
    name: 'Parchment',
    family: 'classic',
    variant: 'light',
    printSafe: true,
    colors: { bg: '#F1E8D0', fg: '#3B2F20', accent: '#7B4A2B' },
  },
  // Futuristic
  {
    id: 'cyberpunk',
    name: 'Cyberpunk',
    family: 'futuristic',
    variant: 'dark',
    printSafe: false,
    colors: { bg: '#0A0A12', fg: '#D0D0E0', accent: '#FF2E8B' },
  },
  {
    id: 'terminal',
    name: 'Terminal',
    family: 'futuristic',
    variant: 'dark',
    printSafe: false,
    colors: { bg: '#0C0C0C', fg: '#33FF33', accent: '#66FF66' },
  },
  {
    id: 'hologram',
    name: 'Hologram',
    family: 'futuristic',
    variant: 'dark',
    printSafe: false,
    colors: { bg: '#0B1628', fg: '#C8DBF0', accent: '#58A6FF' },
  },
  {
    id: 'synthwave',
    name: 'Synthwave',
    family: 'futuristic',
    variant: 'dark',
    printSafe: false,
    colors: { bg: '#1A0A2E', fg: '#E8D0F0', accent: '#FF6EC7' },
  },
  {
    id: 'matrix',
    name: 'Matrix',
    family: 'futuristic',
    variant: 'dark',
    printSafe: false,
    colors: { bg: '#000000', fg: '#00B300', accent: '#00FF41' },
  },
  // Nature
  {
    id: 'forest',
    name: 'Forest',
    family: 'nature',
    variant: 'light',
    printSafe: true,
    colors: { bg: '#F4F2ED', fg: '#1E2B1E', accent: '#2D4A2D' },
  },
  {
    id: 'ocean',
    name: 'Ocean',
    family: 'nature',
    variant: 'dark',
    printSafe: false,
    colors: { bg: '#0D1B2A', fg: '#C5DBE8', accent: '#7EC8C8' },
  },
  {
    id: 'sunset',
    name: 'Sunset',
    family: 'nature',
    variant: 'light',
    printSafe: true,
    colors: { bg: '#FFF8F0', fg: '#3A2218', accent: '#C44B2B' },
  },
  {
    id: 'arctic',
    name: 'Arctic',
    family: 'nature',
    variant: 'light',
    printSafe: true,
    colors: { bg: '#F0F4F8', fg: '#1C2A38', accent: '#2E5080' },
  },
  {
    id: 'sakura',
    name: 'Sakura',
    family: 'nature',
    variant: 'light',
    printSafe: true,
    colors: { bg: '#FDF8F5', fg: '#3A2B30', accent: '#C45C78' },
  },
  // Artistic
  {
    id: 'noir',
    name: 'Noir',
    family: 'artistic',
    variant: 'dark',
    printSafe: false,
    colors: { bg: '#0F0F0F', fg: '#D8D8D8', accent: '#F43030' },
  },
  {
    id: 'candy',
    name: 'Candy',
    family: 'artistic',
    variant: 'light',
    printSafe: true,
    colors: { bg: '#FFF5FA', fg: '#3C2845', accent: '#E04080' },
  },
  {
    id: 'blueprint',
    name: 'Blueprint',
    family: 'artistic',
    variant: 'dark',
    printSafe: false,
    colors: { bg: '#1B3A5C', fg: '#D0E0F0', accent: '#FFFFFF' },
  },
  {
    id: 'witch',
    name: 'Witch',
    family: 'artistic',
    variant: 'dark',
    printSafe: false,
    colors: { bg: '#110E18', fg: '#C8B8D8', accent: '#B040E0' },
  },
  // Greyscale
  {
    id: 'greyscale-warm',
    name: 'Greyscale Warm',
    family: 'greyscale',
    variant: 'light',
    printSafe: true,
    colors: { bg: '#F5F0E8', fg: '#3D3632', accent: '#706252' },
  },
  {
    id: 'greyscale-cool',
    name: 'Greyscale Cool',
    family: 'greyscale',
    variant: 'light',
    printSafe: true,
    colors: { bg: '#EBEEF2', fg: '#2B3038', accent: '#5A6A7A' },
  },
  {
    id: 'high-contrast',
    name: 'High Contrast',
    family: 'greyscale',
    variant: 'light',
    printSafe: true,
    colors: { bg: '#FFFFFF', fg: '#000000', accent: '#000000' },
  },
];

const THEME_MAP = new Map(THEMES.map(t => [t.id, t]));

// ── Mini Document Preview ────────────────────────────────────────

function MiniPagePreview({ colors }: { colors: ThemeMeta['colors'] }) {
  return (
    <div
      className="relative h-12 w-9 shrink-0 overflow-hidden rounded-sm shadow-sm"
      style={{ backgroundColor: colors.bg }}
    >
      {/* "Heading" bar */}
      <div
        className="mx-1.5 mt-1.5 h-[3px] w-3 rounded-full"
        style={{ backgroundColor: colors.accent }}
      />
      {/* "Text" lines */}
      <div
        className="mx-1.5 mt-1 h-[2px] w-5 rounded-full opacity-50"
        style={{ backgroundColor: colors.fg }}
      />
      <div
        className="mx-1.5 mt-0.5 h-[2px] w-4 rounded-full opacity-35"
        style={{ backgroundColor: colors.fg }}
      />
      <div
        className="mx-1.5 mt-0.5 h-[2px] w-[18px] rounded-full opacity-35"
        style={{ backgroundColor: colors.fg }}
      />
      {/* "Code block" */}
      <div
        className="mx-1 mt-1 h-[6px] rounded-[1px] opacity-15"
        style={{ backgroundColor: colors.fg }}
      />
    </div>
  );
}

// ── Theme Card ───────────────────────────────────────────────────

function ThemeCard({
  theme,
  isActive,
  onSelect,
}: {
  theme: ThemeMeta;
  isActive: boolean;
  onSelect: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onSelect}
      className={`group flex w-full items-center gap-3 rounded-xl px-3 py-2.5 text-left transition-all duration-200 ${
        isActive
          ? 'bg-sc-purple/12 ring-1 ring-sc-purple/40 shadow-[0_0_16px_rgba(225,53,255,0.1)]'
          : 'bg-sc-bg-dark/60 hover:bg-sc-bg-highlight/80 ring-1 ring-white/[0.04] hover:ring-white/[0.08]'
      }`}
    >
      <MiniPagePreview colors={theme.colors} />

      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <span
            className={`truncate text-sm font-medium ${
              isActive ? 'text-sc-purple' : 'text-sc-fg group-hover:text-white'
            }`}
          >
            {theme.name}
          </span>
          {isActive && (
            <svg
              aria-hidden="true"
              className="h-3.5 w-3.5 shrink-0 text-sc-purple"
              fill="currentColor"
              viewBox="0 0 20 20"
            >
              <path
                fillRule="evenodd"
                d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z"
                clipRule="evenodd"
              />
            </svg>
          )}
        </div>
        <div className="mt-0.5 flex items-center gap-1.5">
          <span
            className={`inline-flex items-center rounded px-1 py-px text-[10px] font-semibold uppercase tracking-wider ${
              theme.variant === 'light'
                ? 'bg-amber-400/15 text-amber-300'
                : 'bg-indigo-400/15 text-indigo-300'
            }`}
          >
            {theme.variant}
          </span>
          {theme.printSafe && (
            <span className="inline-flex items-center gap-0.5 text-[10px] text-sc-fg-dim">
              <svg
                aria-hidden="true"
                className="h-2.5 w-2.5"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
                strokeWidth={2}
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  d="M17 17h2a2 2 0 002-2v-4a2 2 0 00-2-2H5a2 2 0 00-2 2v4a2 2 0 002 2h2m2 4h6a2 2 0 002-2v-4a2 2 0 00-2-2H9a2 2 0 00-2 2v4a2 2 0 002 2zm8-12V5a2 2 0 00-2-2H9a2 2 0 00-2 2v4h10z"
                />
              </svg>
              print-safe
            </span>
          )}
        </div>
      </div>

      {/* Accent color dot */}
      <div
        className="h-3 w-3 shrink-0 rounded-full ring-1 ring-white/10"
        style={{ backgroundColor: theme.colors.accent }}
      />
    </button>
  );
}

// ── Main Selector Component ──────────────────────────────────────

interface ThemeSelectorProps {
  activeTheme: string;
  onSelect: (themeId: string) => void;
  disabled?: boolean;
}

export function ThemeSelector({ activeTheme, onSelect, disabled }: ThemeSelectorProps) {
  const [expanded, setExpanded] = useState(false);
  const [search, setSearch] = useState('');
  const [activeFamily, setActiveFamily] = useState<string | null>(null);
  const searchRef = useRef<HTMLInputElement>(null);
  const panelRef = useRef<HTMLDivElement>(null);

  const currentTheme = THEME_MAP.get(activeTheme);

  // Focus search when expanding
  useEffect(() => {
    if (expanded) {
      // Small delay to allow animation to start
      const t = setTimeout(() => searchRef.current?.focus(), 100);
      return () => clearTimeout(t);
    }
    // Reset filters on close
    setSearch('');
    setActiveFamily(null);
  }, [expanded]);

  // Close on Escape
  useEffect(() => {
    if (!expanded) return;
    const handler = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setExpanded(false);
    };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, [expanded]);

  // Close on click outside
  useEffect(() => {
    if (!expanded) return;
    const handler = (e: MouseEvent) => {
      if (panelRef.current && !panelRef.current.contains(e.target as Node)) {
        setExpanded(false);
      }
    };
    // Use setTimeout so the opening click doesn't immediately close it
    const t = setTimeout(() => window.addEventListener('mousedown', handler), 0);
    return () => {
      clearTimeout(t);
      window.removeEventListener('mousedown', handler);
    };
  }, [expanded]);

  // Filter themes
  const filteredThemes = useMemo(() => {
    let result = THEMES;

    if (activeFamily) {
      result = result.filter(t => t.family === activeFamily);
    }

    if (search.trim()) {
      const q = search.toLowerCase().trim();
      result = result.filter(
        t =>
          t.name.toLowerCase().includes(q) ||
          t.id.toLowerCase().includes(q) ||
          t.family.toLowerCase().includes(q) ||
          t.variant.toLowerCase().includes(q)
      );
    }

    return result;
  }, [search, activeFamily]);

  // Group by family for display
  const groupedThemes = useMemo(() => {
    if (activeFamily || search.trim()) return null; // flat list when filtered
    const groups: { family: (typeof FAMILIES)[number]; themes: ThemeMeta[] }[] = [];
    for (const fam of FAMILIES) {
      const themes = filteredThemes.filter(t => t.family === fam.id);
      if (themes.length > 0) groups.push({ family: fam, themes });
    }
    return groups;
  }, [filteredThemes, activeFamily, search]);

  const handleSelect = useCallback(
    (id: string) => {
      onSelect(id);
      setExpanded(false);
    },
    [onSelect]
  );

  return (
    <div className="relative mb-6">
      {/* ── Trigger Row ─────────────────────────────────────── */}
      <div className="flex items-center justify-center gap-3">
        {/* Current theme pill */}
        {currentTheme && (
          <div className="flex items-center gap-2.5 rounded-xl bg-sc-bg-dark/80 px-3 py-2 ring-1 ring-white/[0.06]">
            <MiniPagePreview colors={currentTheme.colors} />
            <div>
              <div className="text-sm font-semibold text-sc-fg">{currentTheme.name}</div>
              <div className="flex items-center gap-1.5">
                <span
                  className={`text-[10px] font-semibold uppercase tracking-wider ${
                    currentTheme.variant === 'light' ? 'text-amber-300' : 'text-indigo-300'
                  }`}
                >
                  {currentTheme.variant}
                </span>
                {currentTheme.printSafe && (
                  <span className="text-[10px] text-sc-fg-dim">/ print-safe</span>
                )}
              </div>
            </div>
          </div>
        )}

        {/* Toggle button */}
        <button
          type="button"
          onClick={() => setExpanded(v => !v)}
          disabled={disabled}
          className={`flex items-center gap-2 rounded-xl px-4 py-2.5 text-sm font-medium transition-all ${
            expanded
              ? 'bg-sc-purple/15 text-sc-purple ring-1 ring-sc-purple/30'
              : 'bg-sc-bg-highlight text-sc-fg-muted ring-1 ring-white/[0.06] hover:bg-sc-bg-surface hover:text-sc-fg hover:ring-white/[0.1]'
          } disabled:cursor-not-allowed disabled:opacity-40`}
        >
          <svg
            aria-hidden="true"
            className={`h-4 w-4 transition-transform duration-300 ${expanded ? 'rotate-180' : ''}`}
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
            strokeWidth={2}
          >
            <path strokeLinecap="round" strokeLinejoin="round" d="M19 9l-7 7-7-7" />
          </svg>
          {expanded ? 'Close' : `Browse all ${THEMES.length} themes`}
        </button>
      </div>

      {/* ── Floating Overlay Panel ────────────────────────── */}
      {expanded && (
        <div ref={panelRef} className="absolute left-0 right-0 top-full z-40 pt-3 animate-drop-in">
          <div className="rounded-2xl bg-sc-bg-dark/95 p-4 shadow-[0_16px_48px_rgba(0,0,0,0.4)] ring-1 ring-white/[0.08] backdrop-blur-xl">
            {/* Search */}
            <div className="relative mb-3">
              <svg
                aria-hidden="true"
                className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-sc-fg-dim"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
                strokeWidth={2}
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
                />
              </svg>
              <input
                ref={searchRef}
                type="text"
                placeholder="Search themes..."
                value={search}
                onChange={e => setSearch(e.target.value)}
                className="w-full rounded-xl bg-sc-bg/80 py-2.5 pl-10 pr-4 text-sm text-sc-fg placeholder:text-sc-fg-dim ring-1 ring-white/[0.06] transition-all focus:outline-none focus:ring-sc-purple/40 focus:shadow-[0_0_20px_rgba(225,53,255,0.08)]"
              />
              {search && (
                <button
                  type="button"
                  onClick={() => setSearch('')}
                  className="absolute right-3 top-1/2 -translate-y-1/2 text-sc-fg-dim hover:text-sc-fg"
                >
                  <svg
                    aria-hidden="true"
                    className="h-4 w-4"
                    fill="none"
                    viewBox="0 0 24 24"
                    stroke="currentColor"
                    strokeWidth={2}
                  >
                    <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
                  </svg>
                </button>
              )}
            </div>

            {/* Family filter pills */}
            <div className="mb-3 flex flex-wrap gap-1.5">
              <button
                type="button"
                onClick={() => setActiveFamily(null)}
                className={`rounded-lg px-2.5 py-1 text-xs font-medium transition-all ${
                  activeFamily === null
                    ? 'bg-sc-purple/15 text-sc-purple ring-1 ring-sc-purple/30'
                    : 'text-sc-fg-dim hover:bg-sc-bg-highlight hover:text-sc-fg'
                }`}
              >
                All ({THEMES.length})
              </button>
              {FAMILIES.map(fam => {
                const count = THEMES.filter(t => t.family === fam.id).length;
                return (
                  <button
                    type="button"
                    key={fam.id}
                    onClick={() => setActiveFamily(f => (f === fam.id ? null : fam.id))}
                    className={`rounded-lg px-2.5 py-1 text-xs font-medium transition-all ${
                      activeFamily === fam.id
                        ? 'bg-sc-purple/15 text-sc-purple ring-1 ring-sc-purple/30'
                        : 'text-sc-fg-dim hover:bg-sc-bg-highlight hover:text-sc-fg'
                    }`}
                  >
                    {fam.label} ({count})
                  </button>
                );
              })}
            </div>

            {/* Theme grid */}
            <div className="editor-scrollbar max-h-[360px] overflow-y-auto pr-1">
              {filteredThemes.length === 0 && (
                <div className="py-8 text-center text-sm text-sc-fg-dim">
                  No themes match &ldquo;{search}&rdquo;
                </div>
              )}

              {/* Grouped view (when no filter active) */}
              {groupedThemes?.map(({ family, themes }) => (
                <div key={family.id} className="mb-4 last:mb-0">
                  <div className="mb-2 flex items-center gap-2 px-1">
                    <h4 className="text-xs font-semibold uppercase tracking-widest text-sc-fg-dim">
                      {family.label}
                    </h4>
                    <div className="h-px flex-1 bg-white/[0.04]" />
                  </div>
                  <div className="grid grid-cols-1 gap-1.5 sm:grid-cols-2">
                    {themes.map(theme => (
                      <ThemeCard
                        key={theme.id}
                        theme={theme}
                        isActive={activeTheme === theme.id}
                        onSelect={() => handleSelect(theme.id)}
                      />
                    ))}
                  </div>
                </div>
              ))}

              {/* Flat view (when searching or family selected) */}
              {!groupedThemes && (
                <div className="grid grid-cols-1 gap-1.5 sm:grid-cols-2">
                  {filteredThemes.map(theme => (
                    <ThemeCard
                      key={theme.id}
                      theme={theme}
                      isActive={activeTheme === theme.id}
                      onSelect={() => handleSelect(theme.id)}
                    />
                  ))}
                </div>
              )}
            </div>

            {/* Footer count */}
            {(search || activeFamily) && filteredThemes.length > 0 && (
              <div className="mt-2 text-center text-xs text-sc-fg-dim">
                {filteredThemes.length} of {THEMES.length} themes
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
