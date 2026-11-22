import { Header } from "@/components/Header";
import { Hero } from "@/components/Hero";
import { ArchitectureExplorer } from "@/components/ArchitectureExplorer";
import { Features } from "@/components/Features";
import { DownloadSection } from "@/components/DownloadSection";
import { Footer } from "@/components/Footer";

export default function Home() {
  return (
    <main className="min-h-screen flex flex-col">
      <Header />
      <Hero />
      <ArchitectureExplorer />
      <Features />
      <DownloadSection />
      <Footer />
    </main>
  );
}
