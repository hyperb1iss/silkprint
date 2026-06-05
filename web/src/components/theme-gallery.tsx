'use client';

import { useEffect, useMemo, useState } from 'react';
import type { ThemeInfo } from '@/lib/silkprint';
import { familyOptionsFor, normalizeThemes, type ThemeMeta } from '@/lib/themes';

export function ThemeGallery() {
  const [themes, setThemes] = useState<ThemeInfo[]>([]);

  useEffect(() => {
    let cancelled = false;

    (async () => {
      try {
        const silkprint = await import('@/lib/silkprint');
        const catalog = await silkprint.listThemesDetailed();
        if (!cancelled) setThemes(catalog);
      } catch {
        if (!cancelled) setThemes([]);
      }
    })();

    return () => {
      cancelled = true;
    };
  }, []);

  const themeList = useMemo(() => normalizeThemes(themes), [themes]);
  const familyOptions = useMemo(() => familyOptionsFor(themeList), [themeList]);
  const groups = useMemo(
    () =>
      familyOptions
        .map(family => ({
          family,
          themes: themeList.filter(theme => theme.family === family.id),
        }))
        .filter(group => group.themes.length > 0),
    [familyOptions, themeList]
  );

  return (
    <section id="themes" className="mx-auto max-w-7xl px-4 py-12 sm:px-6 md:py-20">
      <div className="mb-8 text-center md:mb-12">
        <h2 className="mb-2 text-2xl font-bold tracking-tight sm:text-3xl md:mb-3 md:text-4xl">
          <span className="gradient-text">{themeList.length || 40} Themes</span>
        </h2>
        <p className="mx-auto max-w-lg text-sc-fg-muted">
          From developer favorites to print-perfect classics — and every one styles your terminal
          reader and your PDF the same way, with WCAG-checked contrast.
        </p>
      </div>

      {groups.length > 0 ? (
        <div className="space-y-10">
          {groups.map(({ family, themes }) => (
            <div key={family.id}>
              <h3 className="mb-4 text-sm font-semibold uppercase tracking-wider text-sc-fg-dim">
                {family.label}
              </h3>
              <div className="grid grid-cols-2 gap-3 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6">
                {themes.map(theme => (
                  <ThemeCard key={theme.id} theme={theme} />
                ))}
              </div>
            </div>
          ))}
        </div>
      ) : (
        <div aria-hidden="true" className="grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-6">
          {Array.from({ length: 12 }, (_, index) => (
            <div
              key={index}
              className="h-[136px] animate-pulse rounded-xl bg-sc-bg-dark ring-1 ring-white/[0.04]"
            />
          ))}
        </div>
      )}
    </section>
  );
}

function ThemeCard({ theme }: { theme: ThemeMeta }) {
  const isLight = theme.variant === 'light';

  return (
    <button
      type="button"
      className="glow-border group flex flex-col overflow-hidden rounded-xl bg-sc-bg-dark transition-all hover:-translate-y-1"
    >
      <div
        className="relative h-24 p-3"
        style={{
          background: theme.colors.bg || (isLight ? '#ffffff' : '#1a1a2e'),
        }}
      >
        <div className="space-y-1.5">
          <div className="h-2.5 w-3/4 rounded-sm" style={{ background: theme.colors.accent }} />
          <div
            className="h-1.5 w-full rounded-sm"
            style={{
              background: theme.colors.fg,
              opacity: isLight ? 0.14 : 0.18,
            }}
          />
          <div
            className="h-1.5 w-5/6 rounded-sm"
            style={{
              background: theme.colors.fg,
              opacity: isLight ? 0.1 : 0.12,
            }}
          />
          <div
            className="h-1.5 w-4/5 rounded-sm"
            style={{
              background: theme.colors.fg,
              opacity: isLight ? 0.1 : 0.12,
            }}
          />
          <div
            className="mt-2 h-1.5 w-2/3 rounded-sm"
            style={{ background: theme.colors.accent, opacity: 0.6 }}
          />
        </div>
      </div>
      <div className="flex items-center gap-2 px-3 py-2.5">
        <span className="h-2.5 w-2.5 rounded-full" style={{ background: theme.colors.accent }} />
        <span className="truncate text-xs font-medium text-sc-fg-muted group-hover:text-sc-fg">
          {theme.label}
        </span>
      </div>
    </button>
  );
}
