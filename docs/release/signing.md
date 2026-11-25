# Releases, auto-update and code signing

Status 2026-11-25. Apple notarization ($99/yr) is out of scope.

## How a release flows

```
git tag vX.Y.Z && git push --tags
  preflight   updater key present? tag == tauri.conf.json version? notes from CHANGELOG.md
  desktop     Windows / macOS universal / Linux via tauri-action -> DRAFT release
              + *.sig updater signatures + latest.json (signed feed)
              + Linux AppImage GPG-signed (if GPG_PRIVATE_KEY set)
  cli         architekton-cli-* archives
  sign-windows (only if SignPath configured) Authenticode-sign .exe/.msi,
              re-create their updater .sig, patch latest.json
  publish     SHA256SUMS.txt (+ .asc), Sigstore build-provenance attestations,
              draft -> published + marked latest
```

Installed apps read `https://github.com/IamSamk/Architekton/releases/latest/download/latest.json`
4 s after launch and every 6 h, show the pop-up with the CHANGELOG notes, download with a
progress bar, verify the minisign signature against the pubkey in `tauri.conf.json`, install
and relaunch. A draft is invisible to `/releases/latest`, so users never see a half-built release.

To cut a release: bump the version in `Cargo.toml` (workspace), `apps/desktop/src-tauri/tauri.conf.json`,
`apps/desktop/package.json`, `apps/web/package.json`; add a `## [X.Y.Z]` section to `CHANGELOG.md`;
tag and push.

## What is signed today, and what it costs

| Thing | Method | Cost | Needs your account? |
|---|---|---|---|
| Update packages, all OS | Tauri updater minisign key | free | key already made; secret must be set (below) |
| Windows .exe / .msi | **SignPath Foundation** Authenticode (OSS program) | free | yes: apply |
| Linux AppImage | embedded GPG signature (`SIGN=1`) | free | yes: make a GPG key |
| All files | `SHA256SUMS.txt` + GPG `.asc` | free | same GPG key |
| All files | GitHub artifact attestations (Sigstore, keyless) | free | no, works now |
| macOS | not notarized | $99/yr | skipped |

Why not Azure Artifact Signing (ex "Trusted Signing")? About $9.99/month Basic, and identity
validation for individuals is limited to a few countries (US/Canada/EU/UK at last check), so it
is neither free nor likely available from India. SignPath Foundation is the free route for OSS.

Note on SmartScreen: an OV-style certificate (which SignPath Foundation issues) removes the
"Unknown publisher" text, but SmartScreen still builds reputation per certificate over downloads.
The first signed releases can still show "Windows protected your PC" for a while. The publisher
shown will be **SignPath Foundation**, not your name; that is how the free program works.

Linux has no OS-level gatekeeper like SmartScreen. Running an AppImage never warns; GPG signatures
and checksums are for people who want to verify. Debian/RPM repo signing only matters once we host
an apt/dnf repo (to-do below).

## Your to-do (accounts only you can create)

1. **Updater secret (required, 2 min).** The public key in `tauri.conf.json` came from
   `pnpm tauri signer generate`. Put the matching private key and its password in
   GitHub -> Settings -> Secrets and variables -> Actions:
   `TAURI_SIGNING_PRIVATE_KEY` (contents of the `.key` file) and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.
   If you lost the key, generate a new pair, update `plugins.updater.pubkey`, and release. Apps
   installed with the old key will not auto-update across that change; they need one manual install.
   The release workflow fails in `preflight` with a clear message when this is missing.
2. **GPG key (optional, 5 min).**
   ```
   gpg --quick-gen-key "Architekton Releases <your-email>" ed25519 sign 2y
   gpg --armor --export-secret-keys <KEYID>   # -> secret GPG_PRIVATE_KEY
   gpg --armor --export <KEYID> > docs/release/architekton-release.asc   # commit this
   ```
   Add `GPG_PASSPHRASE` too. Publish the fingerprint on the website's code-signing page.
3. **SignPath Foundation (optional, days of waiting).**
   - Turn on 2FA on GitHub (required for every team member).
   - Apply at https://signpath.org/apply with the repo URL. Requirements we already meet: OSI
     license (MIT/Apache, no commercial dual-license), no proprietary code, already released,
     built from source on GitHub Actions, a "Code signing policy" page on the website
     (`/code-signing`) linked from the download section and footer.
   - After approval: install https://github.com/apps/signpath on the repo, add GitHub.com as a
     trusted build system, create an artifact configuration that signs `*.exe` and `*.msi`
     inside the zip, and a signing policy (e.g. `release-signing`) with manual approval.
   - Add secret `SIGNPATH_API_TOKEN` (Submitter role) and variables `SIGNPATH_ORGANIZATION_ID`,
     `SIGNPATH_PROJECT_SLUG`, `SIGNPATH_POLICY_SLUG`. The `sign-windows` job turns on by itself.
   - Every release then waits (up to 6 h) for you to click Approve in SignPath.

## Verify

```
sha256sum -c SHA256SUMS.txt --ignore-missing
gpg --verify SHA256SUMS.txt.asc SHA256SUMS.txt
gh attestation verify Architekton_0.3.2_amd64.AppImage -R IamSamk/Architekton
./Architekton_0.3.2_amd64.AppImage --appimage-signature
Get-AuthenticodeSignature .\Architekton_0.3.2_x64-setup.exe
```

## Sources

- SignPath Foundation terms: https://signpath.org/terms
- SignPath GitHub action: https://github.com/SignPath/github-action-submit-signing-request
- Tauri Linux signing: https://v2.tauri.app/distribute/sign/linux/
- Azure Artifact Signing pricing: https://azure.microsoft.com/pricing/details/artifact-signing/
