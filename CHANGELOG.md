# Changelog

Notes for each version are shown in the in-app update pop-up and on the website's releases page.

## [0.3.1] - 2026-11-25

### Updates
- Auto-update now actually ships: releases include a signed `latest.json` and per-platform signatures, so installed apps find new versions on their own.
- New update pop-up with release notes, a real download progress bar, Later and Skip this version.
- Checks 4 seconds after launch and every 6 hours. Packages are signature-checked before install.
- Windows installs updates in passive mode (progress only, no wizard).

### Trust
- Every release lists `SHA256SUMS.txt` and carries GitHub build-provenance attestations.
- Release pipeline ready for free Windows signing (SignPath Foundation) and GPG-signed Linux AppImages.
