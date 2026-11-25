# 19. node_modules, binaries and DLLs

**Status: next.**

## Dependencies (node_modules, site-packages, vendor, target)

- Read the lockfiles first: `pnpm-lock.yaml`, `package-lock.json`, `uv.lock`,
  `poetry.lock`, `Cargo.lock`, `go.sum`. They give exact versions and the dependency
  graph without walking millions of files.
- Disk size per package comes from one directory walk with `ignore` and
  parallel `jwalk`, capped by time; it shows as a treemap.
- Student view: "Your app imports 14 packages directly; they pull in 812 more. The
  largest is `@next/swc` at 120 MB, a native compiler binary."

## Native binaries (ELF, PE/DLL, Mach-O)

- Parse headers with `object` 0.41 (uniform API) or `goblin` 0.10 (richer PE), memory
  mapped with `memmap2`, never fully read.
- Show:
  - architecture and kind (exe, shared library);
  - imported libraries, giving an edge to each DLL or .so;
  - exported symbol count;
  - sections with their sizes (code, data, resources);
  - whether it is stripped, and its debug-info link.
- The DLL graph is a tier of its own, "Native", in the Architecture view, reached
  through the semantic zoom like any other.
- Safety: parse only, never load or execute. Files over 2 GB are skipped with a note.
