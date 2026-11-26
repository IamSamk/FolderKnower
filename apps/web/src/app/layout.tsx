import type { Metadata, Viewport } from "next";
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

const description =
  "Drop a project folder and see how it is built: tiers, routes, call paths and database tables, each linked to the code that proves it. Runs offline on your computer.";

export const metadata: Metadata = {
  metadataBase: new URL("https://folderknower.vercel.app"),
  title: "FolderKnower: see how any codebase is built",
  description,
  openGraph: {
    title: "FolderKnower",
    description,
    url: "https://folderknower.vercel.app",
    siteName: "FolderKnower",
    locale: "en_US",
    type: "website",
  },
};

export const viewport: Viewport = {
  themeColor: [
    { media: "(prefers-color-scheme: light)", color: "oklch(95% 0.011 275)" },
    { media: "(prefers-color-scheme: dark)", color: "oklch(19.4% 0.036 285)" },
  ],
};

// Sets the saved (or system) theme before first paint, so the page does not flash.
const themeScript = `try{var t=localStorage.getItem("ak-theme");if(t!=="light"&&t!=="dark"){t=matchMedia("(prefers-color-scheme: light)").matches?"light":"dark"}document.documentElement.setAttribute("data-theme",t)}catch(e){}`;

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html
      lang="en"
      data-theme="dark"
      suppressHydrationWarning
      className={`${bricolage.variable} ${jetbrains.variable}`}
    >
      <head>
        {/* biome-ignore lint/security/noDangerouslySetInnerHtml: static theme script, no user input */}
        <script dangerouslySetInnerHTML={{ __html: themeScript }} />
      </head>
      <body className="font-sans antialiased selection:bg-[var(--accent)] selection:text-[var(--on-accent)]">
        {children}
      </body>
    </html>
  );
}
