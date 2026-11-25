# Website to-do (apps/web, Vercel)

Backlog only. Not scheduled. Done items move to CHANGELOG.md.

- [ ] Set `GITHUB_TOKEN` (read-only, public repos) as a Vercel env var so release data never hits the 60 req/h anonymous GitHub limit.
- [ ] OS auto-detect on the download section: show the right installer first, others under "Other platforms".
- [ ] Download counts per release (already in the API data) as a small sparkline on /releases.
- [ ] Publish the release GPG key (`docs/release/architekton-release.asc`) and fingerprint on /code-signing once created.
- [ ] Add SignPath "Free code signing provided by SignPath.io" badge once the application is approved.
- [ ] Fix the CLI card: `cargo install ark-cli` is not published to crates.io; show the `install.sh` / `install.ps1` one-liners instead.
- [ ] Per-release pages (`/releases/v0.3.2`) with full notes, assets, checksums and attestation links, for sharing.
- [ ] RSS/Atom feed of releases.
- [ ] winget / Scoop / Flathub / AUR install snippets once those manifests exist.
- [ ] Hosted apt and dnf repositories (signed with the release GPG key) so Linux users get updates through the system package manager.
- [ ] OpenGraph images per page; real domain instead of the placeholder `architekton.dev` in layout metadata.
- [ ] Student-first docs section: "Read your first architecture map" walkthrough with the demo graph.
- [ ] Lighthouse pass (fonts, images, CLS on the explorer) and an accessibility pass (focus rings, contrast in dark mode).
- [ ] Mirror the visualization milestones as an interactive roadmap page.
