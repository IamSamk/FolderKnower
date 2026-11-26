// Live release data from GitHub, cached by Next (ISR) so the site always shows the
// newest version without a redeploy and without hitting the GitHub rate limit.

export const REPO = "IamSamk/Architekton";
const API = `https://api.github.com/repos/${REPO}/releases`;
const REVALIDATE_SECONDS = 600;

export interface Asset {
  name: string;
  url: string;
  size: number;
  downloads: number;
}

export interface Release {
  tag: string;
  name: string;
  body: string;
  url: string;
  publishedAt: string;
  assets: Asset[];
}

interface GhRelease {
  tag_name: string;
  name: string | null;
  body: string | null;
  html_url: string;
  published_at: string | null;
  draft: boolean;
  prerelease: boolean;
  assets: { name: string; browser_download_url: string; size: number; download_count: number }[];
}

function toRelease(r: GhRelease): Release {
  return {
    tag: r.tag_name,
    name: r.name ?? r.tag_name,
    body: r.body ?? "",
    url: r.html_url,
    publishedAt: r.published_at ?? "",
    assets: r.assets.map((a) => ({
      name: a.name,
      url: a.browser_download_url,
      size: a.size,
      downloads: a.download_count,
    })),
  };
}

async function gh<T>(path: string): Promise<T | null> {
  try {
    const headers: Record<string, string> = { Accept: "application/vnd.github+json" };
    if (process.env.GITHUB_TOKEN) headers.Authorization = `Bearer ${process.env.GITHUB_TOKEN}`;
    const res = await fetch(`${API}${path}`, { headers, next: { revalidate: REVALIDATE_SECONDS } });
    if (!res.ok) return null;
    return (await res.json()) as T;
  } catch {
    return null;
  }
}

export async function getLatestRelease(): Promise<Release | null> {
  const r = await gh<GhRelease>("/latest");
  return r ? toRelease(r) : null;
}

export async function getReleases(): Promise<Release[]> {
  const list = await gh<GhRelease[]>("?per_page=30");
  return (list ?? []).filter((r) => !r.draft).map(toRelease);
}

/** First asset whose name matches, else the release page. */
export function pick(release: Release | null, re: RegExp): Asset | null {
  return release?.assets.find((a) => re.test(a.name)) ?? null;
}

export function fmtSize(bytes: number) {
  return bytes >= 1_048_576
    ? `${(bytes / 1_048_576).toFixed(1)} MB`
    : `${Math.round(bytes / 1024)} KB`;
}

export function fmtDate(iso: string) {
  if (!iso) return "";
  return new Date(iso).toLocaleDateString("en-US", {
    year: "numeric",
    month: "short",
    day: "numeric",
  });
}

export type NoteLine = { id: string; kind: "h" | "li" | "p"; text: string };

/** Minimal, safe markdown reader for release notes (headings, bullets, paragraphs). */
export function noteLines(body: string): NoteLine[] {
  return body
    .split("\n")
    .map((l) => l.trim())
    .filter(Boolean)
    .map((l, i) => {
      const id = `l${i}`;
      const clean = (s: string) => s.replace(/\*\*|__/g, "");
      if (/^#{1,6}\s/.test(l)) return { id, kind: "h", text: clean(l.replace(/^#+\s*/, "")) };
      if (/^[-*]\s/.test(l)) return { id, kind: "li", text: clean(l.slice(2)) };
      return { id, kind: "p", text: clean(l) };
    });
}
