import type { Metadata } from "next";
import { Bricolage_Grotesque, JetBrains_Mono } from "next/font/google";
import "./globals.css";

const bricolage = Bricolage_Grotesque({
  subsets: ["latin"],
  variable: "--font-sans",
  display: "swap",
});

const jetbrains = JetBrains_Mono({
  subsets: ["latin"],
  variable: "--font-mono",
  display: "swap",
});

export const metadata: Metadata = {
  title: "Architekton — See How Any Codebase Is Built",
  description:
    "Offline codebase architecture explorer. Drop a repository and explore its system tiers, subsystems, API flows, call graphs, and database schemas with static evidence.",
  openGraph: {
    title: "Architekton",
    description: "Offline codebase architecture explorer with soft neo-brutalist design.",
    url: "https://architekton.dev",
    siteName: "Architekton",
    locale: "en_US",
    type: "website",
  },
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" data-theme="dark" className={`${bricolage.variable} ${jetbrains.variable}`}>
      <body className="font-sans antialiased selection:bg-[#ffe14d] selection:text-[#1f1d36]">
        {children}
      </body>
    </html>
  );
}
