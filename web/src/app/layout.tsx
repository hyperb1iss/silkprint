import type { Metadata } from 'next';
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

export const metadata: Metadata = {
  title: 'SilkPrint — Markdown to Stunning PDFs',
  description:
    'Transform your Markdown into beautiful, print-ready PDFs with 40+ gorgeous themes. No LaTeX, no setup, no suffering. Just paste and print.',
  keywords: [
    'markdown to pdf',
    'markdown converter',
    'pdf generator',
    'markdown themes',
    'typst',
    'document formatting',
    'print-ready pdf',
    'markdown editor online',
  ],
  authors: [{ name: 'hyperb1iss', url: 'https://github.com/hyperb1iss' }],
  openGraph: {
    title: 'SilkPrint — Markdown to Stunning PDFs',
    description:
      'Transform your Markdown into beautiful, print-ready PDFs with 40+ gorgeous themes.',
    url: 'https://silkprint.md',
    siteName: 'SilkPrint',
    type: 'website',
    locale: 'en_US',
  },
  twitter: {
    card: 'summary_large_image',
    title: 'SilkPrint — Markdown to Stunning PDFs',
    description:
      'Transform your Markdown into beautiful, print-ready PDFs with 40+ gorgeous themes.',
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
