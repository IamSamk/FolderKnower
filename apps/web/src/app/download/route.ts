import { type NextRequest, NextResponse } from "next/server";
import { osFromUserAgent } from "@/lib/os";

// /download sends the visitor to the installer for the OS in their User-Agent.
// Phones and unknown systems go to the download section to pick one.
export function GET(req: NextRequest) {
  const os = osFromUserAgent(req.headers.get("user-agent") ?? "");
  const to = os ? `/download/${os}` : "/#download";
  return NextResponse.redirect(new URL(to, req.url), 302);
}
