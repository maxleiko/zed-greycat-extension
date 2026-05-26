# GreyCat for Zed

Language support for [GreyCat](https://greycat.io) (`.gcl`) in [Zed](https://zed.dev) — completion, diagnostics, formatting, hover, goto-definition, rename, code actions. Backed by [`greycat-analyzer`](https://github.com/maxleiko/greycat-analyzer), which doubles as a CLI linter and formatter for CI.

## Analyzer binary

The extension talks to the `greycat-analyzer` binary over LSP. On first use in a `.gcl` workspace, if the binary isn't already installed, the extension downloads the latest release from GitHub for your platform — Zed surfaces "Checking for update" → "Downloading" in its status UI. The download is per-user and lives inside the extension's working directory.

Subsequent restarts reuse the cached binary without hitting the network. Roughly once per 24 hours, the next restart re-checks GitHub for a newer release and silently upgrades when one is found; the `.last-check.json` sidecar tracks the throttle.

You can also use a binary you installed yourself: put it on `PATH` (`greycat-analyzer --version` should work) or set `lsp.greycat.binary.path` in Zed settings. Discovery order: `lsp.greycat.binary.path` → `PATH` → managed download. First hit wins.

Pre-built binaries exist for Linux x86_64, Apple Silicon macOS, and Windows x86_64. Intel Mac (`darwin/x64`) has no native artifact yet — install manually per the [project README](https://github.com/maxleiko/greycat-analyzer#install).

## Settings

Configure under `lsp.greycat.*` in your Zed settings file.

| Setting | Default | What it does |
| --- | --- | --- |
| `lsp.greycat.binary.path` | — | Absolute path to a `greycat-analyzer` binary. Overrides PATH lookup and the managed download. |
| `lsp.greycat.settings.level` | `info` | LSP server log verbosity. `off` / `info` / `debug` / `trace`. Sets `RUST_LOG` for the analyzer's own crates. |
| `lsp.greycat.initialization_options.lintLibs` | `false` | Surface lint warnings for vendored modules under `lib/<name>/`. Off by default. |
| `lsp.greycat.initialization_options.diagnosticsDebounceMs` | `150` | Debounce window (ms) between full analyzer publishes while you type. |

Example Zed settings:

```json
{
  "lsp": {
    "greycat": {
      "settings": { "level": "info" },
      "initialization_options": {
        "lintLibs": false,
        "diagnosticsDebounceMs": 150
      }
    }
  }
}
```

## Forcing a recheck

The extension throttles update probes to roughly once per 24 hours. To force a fresh check, delete `.last-check.json` from the extension's working directory and restart the language server (`zed: restart language server`).

## Bugs and feedback

Issues for the extension, the analyzer, the formatter, and the LSP server all live in the same place: [github.com/maxleiko/greycat-analyzer/issues](https://github.com/maxleiko/greycat-analyzer/issues).

## License

MIT.
