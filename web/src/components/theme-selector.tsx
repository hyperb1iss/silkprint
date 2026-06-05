'use client';

import { useCallback, useEffect, useMemo, useRef, useState } from 'react';

import type { ThemeInfo } from '@/lib/silkprint';
import { FALLBACK_THEME, familyOptionsFor, normalizeThemes, type ThemeMeta } from '@/lib/themes';

function MiniPagePreview({ colors }: { colors: ThemeMeta['colors'] }) {
  return (
    <div
      className="relative h-12 w-9 shrink-0 overflow-hidden rounded-sm shadow-sm"
      style={{ backgroundColor: colors.bg }}
    >
      <div
        className="mx-1.5 mt-1.5 h-[3px] w-3 rounded-full"
        style={{ backgroundColor: colors.accent }}
      />
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
      <div
        className="mx-1 mt-1 h-[6px] rounded-[1px] opacity-15"
        style={{ backgroundColor: colors.fg }}
      />
    </div>
  );
}

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
            {theme.label}
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

      <div
        className="h-3 w-3 shrink-0 rounded-full ring-1 ring-white/10"
        style={{ backgroundColor: theme.colors.accent }}
      />
    </button>
  );
}

interface ThemeSelectorProps {
  activeTheme: string;
  onSelect: (themeId: string) => void;
  themes: ThemeInfo[];
  disabled?: boolean;
}

export function ThemeSelector({ activeTheme, onSelect, themes, disabled }: ThemeSelectorProps) {
  const [expanded, setExpanded] = useState(false);
  const [search, setSearch] = useState('');
  const [activeFamily, setActiveFamily] = useState<string | null>(null);
  const searchRef = useRef<HTMLInputElement>(null);
  const panelRef = useRef<HTMLDivElement>(null);

  const themeList = useMemo(() => normalizeThemes(themes), [themes]);
  const themeMap = useMemo(() => new Map(themeList.map(theme => [theme.id, theme])), [themeList]);
  const currentTheme = themeMap.get(activeTheme) ?? FALLBACK_THEME;

  const familyOptions = useMemo(() => {
    return familyOptionsFor(themeList);
  }, [themeList]);

  useEffect(() => {
    if (expanded) {
      const t = setTimeout(() => searchRef.current?.focus(), 100);
      return () => clearTimeout(t);
    }

    setSearch('');
    setActiveFamily(null);
  }, [expanded]);

  useEffect(() => {
    if (!expanded) return;
    const handler = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setExpanded(false);
    };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, [expanded]);

  useEffect(() => {
    if (!expanded) return;
    const handler = (e: MouseEvent) => {
      if (panelRef.current && !panelRef.current.contains(e.target as Node)) {
        setExpanded(false);
      }
    };
    const t = setTimeout(() => window.addEventListener('mousedown', handler), 0);
    return () => {
      clearTimeout(t);
      window.removeEventListener('mousedown', handler);
    };
  }, [expanded]);

  const filteredThemes = useMemo(() => {
    let result = themeList;

    if (activeFamily) {
      result = result.filter(t => t.family === activeFamily);
    }

    if (search.trim()) {
      const q = search.toLowerCase().trim();
      result = result.filter(
        t =>
          t.label.toLowerCase().includes(q) ||
          t.id.toLowerCase().includes(q) ||
          t.family.toLowerCase().includes(q) ||
          t.variant.toLowerCase().includes(q)
      );
    }

    return result;
  }, [themeList, search, activeFamily]);

  const groupedThemes = useMemo(() => {
    if (activeFamily || search.trim()) return null;
    const groups: { family: { id: string; label: string }; themes: ThemeMeta[] }[] = [];
    for (const family of familyOptions) {
      const familyThemes = filteredThemes.filter(t => t.family === family.id);
      if (familyThemes.length > 0) groups.push({ family, themes: familyThemes });
    }
    return groups;
  }, [filteredThemes, familyOptions, activeFamily, search]);

  const handleSelect = useCallback(
    (id: string) => {
      onSelect(id);
      setExpanded(false);
    },
    [onSelect]
  );

  const isDisabled = disabled || themeList.length === 0;

  return (
    <div className="relative mb-6">
      <div className="flex items-center justify-center gap-2 sm:gap-3">
        <div className="flex items-center gap-2 rounded-xl bg-sc-bg-dark/80 px-2.5 py-1.5 ring-1 ring-white/[0.06] sm:gap-2.5 sm:px-3 sm:py-2">
          <MiniPagePreview colors={currentTheme.colors} />
          <div>
            <div className="text-xs font-semibold text-sc-fg sm:text-sm">{currentTheme.label}</div>
            <div className="flex items-center gap-1.5">
              <span
                className={`text-[10px] font-semibold uppercase tracking-wider ${
                  currentTheme.variant === 'light' ? 'text-amber-300' : 'text-indigo-300'
                }`}
              >
                {currentTheme.variant}
              </span>
              {currentTheme.printSafe && (
                <span className="hidden text-[10px] text-sc-fg-dim sm:inline">/ print-safe</span>
              )}
            </div>
          </div>
        </div>

        <button
          type="button"
          onClick={() => setExpanded(v => !v)}
          disabled={isDisabled}
          className={`flex items-center gap-1.5 rounded-xl px-3 py-2 text-xs font-medium transition-all sm:gap-2 sm:px-4 sm:py-2.5 sm:text-sm ${
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
          <span className="hidden sm:inline">
            {expanded ? 'Close' : `Browse all ${themeList.length || 40} themes`}
          </span>
          <span className="sm:hidden">{expanded ? 'Close' : 'Themes'}</span>
        </button>
      </div>

      {expanded && (
        <div
          ref={panelRef}
          className="absolute left-0 right-0 top-full z-40 pt-2 animate-drop-in sm:pt-3"
        >
          <div className="rounded-xl bg-sc-bg-dark/95 p-3 shadow-[0_16px_48px_rgba(0,0,0,0.4)] ring-1 ring-white/[0.08] backdrop-blur-xl sm:rounded-2xl sm:p-4">
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

            <div className="mb-3 flex gap-1.5 overflow-x-auto pb-1 sm:flex-wrap sm:overflow-x-visible sm:pb-0">
              <button
                type="button"
                onClick={() => setActiveFamily(null)}
                className={`rounded-lg px-2.5 py-1 text-xs font-medium transition-all ${
                  activeFamily === null
                    ? 'bg-sc-purple/15 text-sc-purple ring-1 ring-sc-purple/30'
                    : 'text-sc-fg-dim hover:bg-sc-bg-highlight hover:text-sc-fg'
                }`}
              >
                All ({themeList.length})
              </button>
              {familyOptions.map(family => {
                const count = themeList.filter(t => t.family === family.id).length;
                return (
                  <button
                    type="button"
                    key={family.id}
                    onClick={() => setActiveFamily(f => (f === family.id ? null : family.id))}
                    className={`rounded-lg px-2.5 py-1 text-xs font-medium transition-all ${
                      activeFamily === family.id
                        ? 'bg-sc-purple/15 text-sc-purple ring-1 ring-sc-purple/30'
                        : 'text-sc-fg-dim hover:bg-sc-bg-highlight hover:text-sc-fg'
                    }`}
                  >
                    {family.label} ({count})
                  </button>
                );
              })}
            </div>

            <div className="editor-scrollbar max-h-[50vh] overflow-y-auto pr-1 sm:max-h-[360px]">
              {filteredThemes.length === 0 && (
                <div className="py-8 text-center text-sm text-sc-fg-dim">
                  No themes match &ldquo;{search}&rdquo;
                </div>
              )}

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

            {(search || activeFamily) && filteredThemes.length > 0 && (
              <div className="mt-2 text-center text-xs text-sc-fg-dim">
                {filteredThemes.length} of {themeList.length} themes
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
