const THEME_FAMILIES = [
  {
    family: 'Signature',
    themes: [
      { name: 'Silk Light', variant: 'light', accent: '#6366f1' },
      { name: 'Silk Dark', variant: 'dark', accent: '#818cf8' },
      { name: 'Manuscript', variant: 'light', accent: '#92400e' },
      { name: 'Monochrome', variant: 'light', accent: '#404040' },
    ],
  },
  {
    family: 'SilkCircuit',
    themes: [
      { name: 'Neon', variant: 'dark', accent: '#e135ff' },
      { name: 'Vibrant', variant: 'dark', accent: '#ff00ff' },
      { name: 'Soft', variant: 'dark', accent: '#e892ff' },
      { name: 'Glow', variant: 'dark', accent: '#00ffff' },
      { name: 'Dawn', variant: 'light', accent: '#7e2bd5' },
    ],
  },
  {
    family: 'Developer',
    themes: [
      { name: 'Nord', variant: 'dark', accent: '#88c0d0' },
      { name: 'Dracula', variant: 'dark', accent: '#bd93f9' },
      { name: 'Catppuccin Mocha', variant: 'dark', accent: '#cba6f7' },
      { name: 'Tokyo Night', variant: 'dark', accent: '#7aa2f7' },
      { name: 'Rose Pine', variant: 'dark', accent: '#c4a7e7' },
      { name: 'Gruvbox Dark', variant: 'dark', accent: '#fabd2f' },
    ],
  },
  {
    family: 'Classic',
    themes: [
      { name: 'Academic', variant: 'light', accent: '#1e3a5f' },
      { name: 'Typewriter', variant: 'light', accent: '#333333' },
      { name: 'Newspaper', variant: 'light', accent: '#1a1a1a' },
      { name: 'Parchment', variant: 'light', accent: '#5c4033' },
    ],
  },
  {
    family: 'Futuristic',
    themes: [
      { name: 'Cyberpunk', variant: 'dark', accent: '#ff2a6d' },
      { name: 'Terminal', variant: 'dark', accent: '#33ff33' },
      { name: 'Hologram', variant: 'dark', accent: '#00e5ff' },
      { name: 'Synthwave', variant: 'dark', accent: '#ff77e9' },
      { name: 'Matrix', variant: 'dark', accent: '#00ff41' },
    ],
  },
  {
    family: 'Nature',
    themes: [
      { name: 'Forest', variant: 'dark', accent: '#4ade80' },
      { name: 'Ocean', variant: 'dark', accent: '#38bdf8' },
      { name: 'Sunset', variant: 'light', accent: '#f97316' },
      { name: 'Arctic', variant: 'light', accent: '#93c5fd' },
      { name: 'Sakura', variant: 'light', accent: '#f472b6' },
    ],
  },
];

export function ThemeGallery() {
  return (
    <section id="themes" className="mx-auto max-w-7xl px-4 py-12 sm:px-6 md:py-20">
      <div className="mb-8 text-center md:mb-12">
        <h2 className="mb-2 text-2xl font-bold tracking-tight sm:text-3xl md:mb-3 md:text-4xl">
          <span className="gradient-text">40+ Themes</span>
        </h2>
        <p className="mx-auto max-w-lg text-sc-fg-muted">
          From developer favorites to print-perfect classics. Every theme is built with WCAG
          contrast compliance and pixel-perfect typography.
        </p>
      </div>

      <div className="space-y-10">
        {THEME_FAMILIES.map(family => (
          <div key={family.family}>
            <h3 className="mb-4 text-sm font-semibold uppercase tracking-wider text-sc-fg-dim">
              {family.family}
            </h3>
            <div className="grid grid-cols-2 gap-3 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6">
              {family.themes.map(theme => (
                <ThemeCard key={theme.name} theme={theme} />
              ))}
            </div>
          </div>
        ))}
      </div>
    </section>
  );
}

function ThemeCard({ theme }: { theme: { name: string; variant: string; accent: string } }) {
  const isLight = theme.variant === 'light';

  return (
    <button
      type="button"
      className="glow-border group flex flex-col overflow-hidden rounded-xl bg-sc-bg-dark transition-all hover:-translate-y-1"
    >
      {/* Mini preview */}
      <div
        className="relative h-24 p-3"
        style={{
          background: isLight ? '#ffffff' : '#1a1a2e',
        }}
      >
        {/* Fake document lines */}
        <div className="space-y-1.5">
          <div className="h-2.5 w-3/4 rounded-sm" style={{ background: theme.accent }} />
          <div
            className="h-1.5 w-full rounded-sm"
            style={{
              background: isLight ? 'rgba(0,0,0,0.12)' : 'rgba(255,255,255,0.15)',
            }}
          />
          <div
            className="h-1.5 w-5/6 rounded-sm"
            style={{
              background: isLight ? 'rgba(0,0,0,0.08)' : 'rgba(255,255,255,0.1)',
            }}
          />
          <div
            className="h-1.5 w-4/5 rounded-sm"
            style={{
              background: isLight ? 'rgba(0,0,0,0.08)' : 'rgba(255,255,255,0.1)',
            }}
          />
          <div
            className="mt-2 h-1.5 w-2/3 rounded-sm"
            style={{ background: theme.accent, opacity: 0.6 }}
          />
        </div>
      </div>
      {/* Label */}
      <div className="flex items-center gap-2 px-3 py-2.5">
        <span className="h-2.5 w-2.5 rounded-full" style={{ background: theme.accent }} />
        <span className="truncate text-xs font-medium text-sc-fg-muted group-hover:text-sc-fg">
          {theme.name}
        </span>
      </div>
    </button>
  );
}
