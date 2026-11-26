import { ArchitectureExplorer } from "@/components/ArchitectureExplorer";
import { DownloadSection } from "@/components/DownloadSection";
import { Features } from "@/components/Features";
import { Footer } from "@/components/Footer";
import { Header } from "@/components/Header";
import { Hero } from "@/components/Hero";

export default function Home() {
  return (
    <main className="min-h-dvh flex flex-col">
      <Header />
      <Hero />
      <ArchitectureExplorer />
      <Features />
      <DownloadSection />
      <Footer />
    </main>
  );
}
