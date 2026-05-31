import type { Metadata, Viewport } from 'next';
import { Inter, JetBrains_Mono } from 'next/font/google';
import './globals.css';

const inter = Inter({
  variable: '--font-inter',
  subsets: ['latin'],
  display: 'swap',
});

const jetbrainsMono = JetBrains_Mono({
  variable: '--font-jetbrains-mono',
  subsets: ['latin'],
  display: 'swap',
});

export const viewport: Viewport = {
  width: 'device-width',
  initialScale: 1,
  maximumScale: 1,
  viewportFit: 'cover',
};

const TITLE = 'SilkPrint — Read Markdown in Your Terminal, or Render a Stunning PDF';
const DESCRIPTION =
  'A rich terminal reader and publication-ready PDFs from one themed engine. Inline images, mermaid diagrams, live theme switching, and 40 gorgeous themes that drive both.';

export const metadata: Metadata = {
  title: TITLE,
  description: DESCRIPTION,
  keywords: [
    'markdown terminal reader',
    'terminal markdown viewer',
    'markdown TUI',
    'markdown to pdf',
    'terminal images kitty sixel',
    'mermaid terminal',
    'markdown themes',
    'typst',
    'ratatui',
    'syntax highlighting terminal',
  ],
  authors: [{ name: 'Hyperbliss Technologies', url: 'https://hyperbliss.tech' }],
  openGraph: {
    title: TITLE,
    description: DESCRIPTION,
    url: 'https://silkprint.md',
    siteName: 'SilkPrint',
    type: 'website',
    locale: 'en_US',
    images: [
      { url: '/reader/hero.png', width: 1904, height: 1360, alt: 'SilkPrint terminal reader' },
    ],
  },
  twitter: {
    card: 'summary_large_image',
    title: TITLE,
    description: DESCRIPTION,
    images: ['/reader/hero.png'],
  },
  robots: {
    index: true,
    follow: true,
  },
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en" className="dark">
      <body className={`${inter.variable} ${jetbrainsMono.variable} antialiased`}>{children}</body>
    </html>
  );
}
