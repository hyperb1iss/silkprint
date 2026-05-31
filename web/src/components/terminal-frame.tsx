import type { ReactNode } from 'react';

/**
 * A faux macOS-style terminal window: traffic-light dots, a mono command
 * label, and a neon-shadowed body. Wraps the reader screenshots/GIF so they
 * read as "this is a real terminal," not a flat image.
 */
export function TerminalFrame({
  label,
  children,
  className = '',
  glow = false,
}: {
  label: string;
  children: ReactNode;
  className?: string;
  glow?: boolean;
}) {
  return (
    <div
      className={`group overflow-hidden rounded-xl border border-sc-purple/20 bg-sc-bg-dark transition-all duration-300 ${
        glow
          ? 'shadow-[0_40px_120px_-30px_rgba(225,53,255,0.45)]'
          : 'shadow-[0_20px_60px_-25px_rgba(0,0,0,0.8)] hover:border-sc-cyan/30 hover:shadow-[0_30px_80px_-30px_rgba(128,255,234,0.25)]'
      } ${className}`}
    >
      <div className="flex items-center gap-2 border-b border-white/5 bg-sc-bg-surface/70 px-4 py-2.5">
        <span className="h-3 w-3 rounded-full bg-[#ff5f57]" />
        <span className="h-3 w-3 rounded-full bg-[#febc2e]" />
        <span className="h-3 w-3 rounded-full bg-[#28c840]" />
        <span className="ml-3 truncate font-mono text-xs text-sc-fg-dim">{label}</span>
      </div>
      <div className="bg-[#12101a]">{children}</div>
    </div>
  );
}
