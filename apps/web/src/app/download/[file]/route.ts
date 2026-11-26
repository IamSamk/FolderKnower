import { type NextRequest, NextResponse } from "next/server";
import { getLatestRelease, INSTALLERS, isInstallerKey, pick } from "@/lib/releases";

// /download/<key> redirects straight to that installer file in the newest published
// release, so the browser starts the download at once. If no release has the file yet,
// the visitor goes back to the download section, which says the build is on its way.
export async function GET(req: NextRequest, ctx: { params: Promise<{ file: string }> }) {
  const { file } = await ctx.params;
  if (!isInstallerKey(file)) {
    return NextResponse.redirect(new URL("/#download", req.url), 302);
  }
  const asset = pick(await getLatestRelease(), INSTALLERS[file]);
  if (!asset) {
    return NextResponse.redirect(new URL("/#download", req.url), 302);
  }
  return NextResponse.redirect(asset.url, 302);
}
