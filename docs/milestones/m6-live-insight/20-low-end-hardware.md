# 20. Low-end hardware budget

**Status: next** (budgets set; measuring is the next slice).

Reference machine: 4 cores, 8 GB RAM, integrated GPU, SATA SSD (a school lab PC).

| Work | Budget on the reference machine | How |
|---|---|---|
| Analyse a 5k-file repo | < 20 s cold, < 2 s warm | tree-sitter in parallel (rayon, `num_cpus - 1`), content-hash cache in SQLite |
| Pattern recognition | < 5% of analysis time | regexes compiled once (`LazyLock`), only on files with matching imports |
| Live sampling | < 1% of one core | 1.5 s interval; process command lines read once (`UpdateKind::OnlyIfNotSet`) |
| Desktop RAM | < 350 MB idle, < 600 MB on a 50k-file repo | level-at-a-time graph (doc 18), no source text kept in memory |
| Built-in narrator | runs on CPU in 1.2 GB | Qwen2.5-0.5B-Instruct Q4, loaded only when asked |
| Animations | respect `prefers-reduced-motion` | motion's reduced-motion hook switches springs to fades |

## Degrade gracefully

- No GPU: WebGL paths fall back to canvas 2D with lower LOD thresholds.
- Under 8 GB: the narrator is off by default, and the setup card says why.
- Slow disk: the progress narration shows which stage is slow ("Parsing 4,812 files")
  so the student knows it is not stuck.

## Measuring

A planned `architekton bench <repo>` command will print stage timings and peak RSS from `ark-telemetry`
on itself; CI will run it on two fixture repos and fail on a regression over 20%.
