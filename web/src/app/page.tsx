import { Editor } from '@/components/editor';
import { Features } from '@/components/features';
import { Footer } from '@/components/footer';
import { Header } from '@/components/header';
import { Hero } from '@/components/hero';
import { ThemeGallery } from '@/components/theme-gallery';

export default function Home() {
  return (
    <div className="min-h-screen bg-sc-bg">
      <Header />
      <Hero />
      <Editor />
      <Features />
      <ThemeGallery />
      <Footer />
    </div>
  );
}
