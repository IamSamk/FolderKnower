export type Os = "windows" | "macos" | "linux";

/** Best guess of the desktop OS from a User-Agent string. Null for phones and unknowns. */
export function osFromUserAgent(ua: string): Os | null {
  if (/android|iphone|ipad|ipod/i.test(ua)) return null;
  if (/windows/i.test(ua)) return "windows";
  if (/mac os x|macintosh/i.test(ua)) return "macos";
  if (/linux|x11/i.test(ua)) return "linux";
  return null;
}
