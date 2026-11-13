# Third-party notices

Vitrine is MIT licensed. It depends on the following projects, whose licences
are reproduced or referenced here. This file is updated as dependencies are
added; run `npm run notices` (planned) to regenerate the JavaScript section.

## Runtime — Rust

| Crate | Licence |
|---|---|
| tauri | MIT / Apache-2.0 |
| tokio | MIT |
| sqlx | MIT / Apache-2.0 |
| serde, serde_json | MIT / Apache-2.0 |
| uuid | Apache-2.0 / MIT |
| chrono | MIT / Apache-2.0 |
| reqwest | MIT / Apache-2.0 |
| tracing | MIT |
| keyring | MIT / Apache-2.0 |
| steamlocate | MIT |
| walkdir | MIT / Unlicense |
| notify | CC0-1.0 |
| sysinfo | MIT |
| image | MIT / Apache-2.0 |
| windows (crate) | MIT / Apache-2.0 |
| nix | MIT |

## Runtime — JavaScript

| Package | Licence |
|---|---|
| react, react-dom | MIT |
| three | MIT |
| @tanstack/react-query | MIT |
| @tanstack/react-virtual | MIT |
| zustand | MIT |
| tailwindcss | MIT |
| @radix-ui/* | MIT |

## Fonts

Served locally; all SIL Open Font License 1.1.

| Font | Licence | Source |
|---|---|---|
| Fraunces | OFL-1.1 | Google Fonts |
| Inter Tight | OFL-1.1 | Google Fonts |
| Geist Mono | OFL-1.1 | Vercel |

## Reference material (not included)

The interaction concept — games presented as physical cases on a shelf, which
can be pulled out, flipped open, and inspected — was informed by
[Boxshelf](https://github.com/sassiestrhihno/boxshelf-source), which is
GPL-3.0. **No code, stylesheet, shader, image, or font file from that project is
used in Vitrine.** See [ADR-0002](docs/adr/0002-own-code-not-a-fork.md).

Console case proportions in `crates/core/migrations/0001_init.sql` are
real-world physical measurements of retail packaging.
