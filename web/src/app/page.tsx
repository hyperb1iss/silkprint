import { Editor } from '@/components/editor';
import { Features } from '@/components/features';
import { Footer } from '@/components/footer';
import { Header } from '@/components/header';
import { Hero } from '@/components/hero';
import { TerminalReader } from '@/components/terminal-reader';
import { ThemeGallery } from '@/components/theme-gallery';

export default function Home() {
  return (
    <div className="min-h-screen overflow-x-hidden bg-sc-bg">
      <Header />
      <Hero />
      <TerminalReader />
      <section id="pdf" className="scroll-mt-20">
        <div className="mx-auto max-w-7xl px-4 pt-8 text-center sm:px-6">
          <p className="mb-3 font-mono text-sm tracking-wider text-sc-coral">silkprint pdf</p>
          <h2 className="text-3xl font-bold tracking-tight sm:text-4xl md:text-5xl">
            Or render a <span className="gradient-text">stunning PDF</span>
          </h2>
          <p className="mx-auto mt-3 max-w-2xl text-sc-fg-muted">
            Paste Markdown, pick a theme, get a print-ready PDF in your browser — the same engine,
            compiled to WebAssembly.
          </p>
        </div>
      </section>
      <Editor />
      <Features />
      <ThemeGallery />
      <Footer />
    </div>
  );
}
