use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use zed_extension_api::lsp::{Completion, CompletionKind};
use zed_extension_api::settings::LspSettings;
use zed_extension_api::*;

struct GreyCatExtension;

const DEFAULT_LEVEL: &str = "info";

/// Repo path under github.com that ships the analyzer binaries.
/// Matches the pattern used by VS Code (`editors/code/src/install.ts`):
/// per-platform assets live at
/// `https://github.com/<REPO>/releases/latest/download/<asset>.zip`.
const ANALYZER_REPO: &str = "maxleiko/greycat-analyzer";

/// Throttle window for the periodic update probe. Zed extensions don't
/// have a background timer affordance, so this is the closest
/// equivalent to VS Code's per-day check: when `language_server_command`
/// is invoked within this window since the last successful check, we
/// reuse the cached binary without calling `latest_github_release`.
const UPDATE_CHECK_INTERVAL_MS: u128 = 24 * 60 * 60 * 1000;

/// Sidecar file used to remember the last update check. Lives next to
/// the downloaded `greycat-analyzer-<tag>/` directories. Schema:
/// `{ "tag": "v0.1.4", "at_ms": 1700000000000 }`.
const LAST_CHECK_FILE: &str = ".last-check.json";

fn resolve_level(settings: Option<&serde_json::Value>) -> &'static str {
    let Some(value) = settings
        .and_then(|v| v.get("level"))
        .and_then(|v| v.as_str())
    else {
        return DEFAULT_LEVEL;
    };
    match value {
        "off" => "off",
        "info" => "info",
        "debug" => "debug",
        "trace" => "trace",
        _ => DEFAULT_LEVEL,
    }
}

fn build_rust_log(level: &str) -> String {
    if level == "off" {
        return "off".into();
    }
    [
        format!("greycat_analyzer_server={level}"),
        format!("greycat_analyzer_core={level}"),
        format!("greycat_analyzer_analysis={level}"),
    ]
    .join(",")
}

fn build_env(level: &str) -> Vec<(String, String)> {
    vec![
        ("RUST_BACKTRACE".into(), "1".into()),
        ("RUST_LOG".into(), build_rust_log(level)),
    ]
}

impl Extension for GreyCatExtension {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self
    }

    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Command> {
        let lsp_settings =
            LspSettings::for_worktree(language_server_id.as_ref(), worktree).unwrap_or_default();
        let level = resolve_level(lsp_settings.settings.as_ref());
        let env = build_env(level);

        // 1. User-supplied path via `lsp.greycat.binary.path`. The Zed
        //    convention for "I want to use my own binary." When set,
        //    trust it verbatim — we don't probe `--version` here
        //    because the WASM sandbox can't spawn arbitrary processes.
        if let Some(binary) = lsp_settings.binary.as_ref()
            && let Some(path) = binary.path.as_ref()
            && !path.is_empty()
        {
            let args = binary
                .arguments
                .clone()
                .unwrap_or_else(|| vec!["server".into()]);
            return Ok(Command {
                command: path.clone(),
                args,
                env,
            });
        }

        // 2. PATH lookup. Users who installed via the analyzer's CLI
        //    install (curl + chmod) stay on their existing binary —
        //    same as VS Code's precedence.
        if let Some(command) = worktree.which("greycat-analyzer") {
            return Ok(Command {
                command,
                args: vec!["server".into()],
                env,
            });
        }

        // 3. Managed install. Resolve the latest tag (subject to a 24h
        //    throttle), download the matching asset if we don't have it
        //    yet, return the path inside the versioned dir.
        let binary_path = ensure_managed_binary(language_server_id)?;
        Ok(Command {
            command: binary_path,
            args: vec!["server".into()],
            env,
        })
    }

    fn language_server_initialization_options(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Option<serde_json::Value>> {
        Ok(
            LspSettings::for_worktree(language_server_id.as_ref(), worktree)
                .unwrap_or_default()
                .initialization_options,
        )
    }

    fn label_for_completion(
        &self,
        _language_server_id: &LanguageServerId,
        completion: Completion,
    ) -> Option<CodeLabel> {
        // The LSP server emits source-shaped strings in `detail` so
        // Zed's tree-sitter pass can apply highlights from the
        // GreyCat `highlights.scm` query. For each completion kind
        // we synthesize a complete parseable fragment as `code` and
        // declare display spans that hide the synthetic prefix.
        let kind = completion.kind?;
        let name = &completion.label;
        match kind {
            // Fns / methods — `detail` is the compact `(args): Ret`
            // form. Wrap as `fn name(args): Ret` so the name is
            // recognized as a `fn_decl` name, args as `fn_param`s,
            // types as `type_ident`s.
            CompletionKind::Function | CompletionKind::Method => {
                let detail = completion.detail.as_deref()?;
                if !detail.starts_with('(') {
                    return None;
                }
                let code = format!("fn {name}{detail}");
                let name_start = "fn ".len() as u32;
                let name_end = name_start + name.len() as u32;
                let detail_end = code.len() as u32;
                Some(CodeLabel {
                    spans: vec![
                        CodeLabelSpan::code_range(name_start..name_end),
                        CodeLabelSpan::code_range(name_end..detail_end),
                    ],
                    filter_range: (0..(name.len() as u32)).into(),
                    code,
                })
            }
            // Types / enums — `detail` is the home module's stem
            // (e.g. `runtime`). Wrap as `type Name {}` / `enum
            // Name {}` so the name renders in `@type.definition`
            // color, then append the module label dimmed via the
            // `@comment` highlight.
            CompletionKind::Class | CompletionKind::Enum => {
                let module = completion.detail.as_deref().unwrap_or("");
                let keyword = if matches!(kind, CompletionKind::Class) {
                    "type"
                } else {
                    "enum"
                };
                let code = format!("{keyword} {name} {{}}");
                let name_start = (keyword.len() + 1) as u32;
                let name_end = name_start + name.len() as u32;
                let mut spans = vec![CodeLabelSpan::code_range(name_start..name_end)];
                if !module.is_empty() {
                    spans.push(CodeLabelSpan::literal("  ", None));
                    spans.push(CodeLabelSpan::literal(module, Some("comment".into())));
                }
                Some(CodeLabel {
                    spans,
                    filter_range: (0..(name.len() as u32)).into(),
                    code,
                })
            }
            // Type attributes (instance + static) — `detail` is the
            // compact `: T` form. Wrap as `type _ { name: T; }` for
            // instance attrs and `type _ { static name: T; }` for
            // statics, so tree-sitter highlights the name as
            // `@field` / `@variable.member.static` and `T` as a
            // type ident.
            CompletionKind::Field | CompletionKind::Constant => {
                let detail = completion.detail.as_deref()?;
                if !detail.starts_with(':') {
                    return None;
                }
                let static_prefix = if matches!(kind, CompletionKind::Constant) {
                    "static "
                } else {
                    ""
                };
                let code = format!("type _ {{ {static_prefix}{name}{detail}; }}");
                let prefix_len = "type _ { ".len() + static_prefix.len();
                let name_start = prefix_len as u32;
                let name_end = name_start + name.len() as u32;
                let detail_end = name_end + detail.len() as u32;
                Some(CodeLabel {
                    spans: vec![
                        CodeLabelSpan::code_range(name_start..name_end),
                        CodeLabelSpan::code_range(name_end..detail_end),
                    ],
                    filter_range: (0..(name.len() as u32)).into(),
                    code,
                })
            }
            _ => None,
        }
    }
}

register_extension!(GreyCatExtension);

// ----------------------------------------------------------------------------
// Managed install
// ----------------------------------------------------------------------------

/// Asset filename for the current platform, e.g.
/// `greycat-analyzer-aarch64-apple-darwin.zip`. Returns `Err` for
/// platforms with no native artifact (today: Intel Mac).
fn asset_filename_for_platform() -> Result<&'static str> {
    let (os, arch) = current_platform();
    match (os, arch) {
        (Os::Linux, Architecture::X8664) => {
            Ok("greycat-analyzer-x86_64-unknown-linux-gnu.zip")
        }
        (Os::Mac, Architecture::Aarch64) => {
            Ok("greycat-analyzer-aarch64-apple-darwin.zip")
        }
        (Os::Windows, Architecture::X8664) => {
            Ok("greycat-analyzer-x86_64-pc-windows-msvc.zip")
        }
        _ => Err(format!(
            "no prebuilt greycat-analyzer artifact for {os:?}/{arch:?}; install manually — see https://github.com/maxleiko/greycat-analyzer#install"
        )),
    }
}

/// Name of the binary inside the extracted zip.
fn binary_filename() -> &'static str {
    match current_platform().0 {
        Os::Windows => "greycat-analyzer.exe",
        _ => "greycat-analyzer",
    }
}

/// Resolve (or install) the managed `greycat-analyzer` binary, returning
/// its absolute path. Looks at the `.last-check.json` sidecar first; if
/// the cached tag's binary is present and the cache is fresh (< 24h),
/// returns immediately without hitting the network. Otherwise queries
/// `latest_github_release`, downloads the asset if it isn't already
/// present under `greycat-analyzer-<tag>/`, prunes other versions, and
/// returns the new path.
///
/// On network failure with a usable cached binary, falls back to the
/// cached binary rather than erroring out — the user keeps a working
/// LSP even when offline.
fn ensure_managed_binary(language_server_id: &LanguageServerId) -> Result<String> {
    let asset_name = asset_filename_for_platform()?;
    let binary_name = binary_filename();
    let cached = read_last_check();

    // Fast path: cache is fresh and the binary it claims is still on disk.
    if let Some(ref last) = cached
        && let Some(tag) = last.tag.as_deref()
        && !is_stale(last.at_ms)
    {
        let dir = version_dir(tag);
        let binary = dir.join(binary_name);
        if binary.exists() {
            return path_string(&binary);
        }
    }

    set_language_server_installation_status(
        language_server_id,
        &LanguageServerInstallationStatus::CheckingForUpdate,
    );

    // Talk to GitHub. On failure, surface the cached binary if we have
    // one; otherwise propagate the error.
    let release_result = latest_github_release(
        ANALYZER_REPO,
        GithubReleaseOptions {
            require_assets: true,
            pre_release: false,
        },
    );
    let release = match release_result {
        Ok(release) => release,
        Err(err) => {
            if let Some(ref last) = cached
                && let Some(tag) = last.tag.as_deref()
            {
                let dir = version_dir(tag);
                let binary = dir.join(binary_name);
                if binary.exists() {
                    set_language_server_installation_status(
                        language_server_id,
                        &LanguageServerInstallationStatus::None,
                    );
                    eprintln!(
                        "[greycat] update check failed ({err}); using cached binary {tag}"
                    );
                    return path_string(&binary);
                }
            }
            set_language_server_installation_status(
                language_server_id,
                &LanguageServerInstallationStatus::Failed(format!(
                    "could not fetch latest release: {err}"
                )),
            );
            return Err(format!("greycat-analyzer install failed: {err}"));
        }
    };

    let tag = release.version.clone();
    let dir = version_dir(&tag);
    let binary = dir.join(binary_name);

    if !binary.exists() {
        let asset_url = release
            .assets
            .iter()
            .find(|a| a.name == asset_name)
            .map(|a| a.download_url.clone())
            .ok_or_else(|| {
                format!(
                    "release {tag} does not contain expected asset `{asset_name}`. Available: {}",
                    release
                        .assets
                        .iter()
                        .map(|a| a.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })?;

        set_language_server_installation_status(
            language_server_id,
            &LanguageServerInstallationStatus::Downloading,
        );
        let dir_str = path_string(&dir)?;
        download_file(&asset_url, &dir_str, DownloadedFileType::Zip)
            .map_err(|err| format!("download of {asset_url} failed: {err}"))?;

        if !binary.exists() {
            return Err(format!(
                "zip `{asset_name}` did not contain expected binary `{binary_name}`"
            ));
        }
        let binary_str = path_string(&binary)?;
        make_file_executable(&binary_str)
            .map_err(|err| format!("could not mark {binary_str} executable: {err}"))?;
        prune_other_versions(&tag);
    }

    write_last_check(&LastCheck {
        tag: Some(tag.clone()),
        at_ms: now_ms(),
    });
    set_language_server_installation_status(
        language_server_id,
        &LanguageServerInstallationStatus::None,
    );

    path_string(&binary)
}

fn version_dir(tag: &str) -> PathBuf {
    // Tag often comes back with a `v` prefix (`v0.1.4`); keep it in
    // the directory name so the on-disk layout matches the
    // `/releases/tag/<tag>` shape GitHub uses.
    PathBuf::from(format!("greycat-analyzer-{tag}"))
}

/// Convert a [`Path`] to a `String`. Returns an error rather than
/// silently lossy-converting because Zed's WIT-level API takes
/// `String` and bad UTF-8 in a path would manifest later as a download
/// or spawn failure with a less informative message.
fn path_string(path: &Path) -> Result<String> {
    path.to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| format!("path {path:?} is not valid UTF-8"))
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

fn is_stale(at_ms: u128) -> bool {
    let now = now_ms();
    now < at_ms || now - at_ms > UPDATE_CHECK_INTERVAL_MS
}

/// Sidecar serialisation. Hand-rolled JSON because pulling in
/// `serde_derive` for two scalars isn't worth the wasm-size cost.
struct LastCheck {
    tag: Option<String>,
    at_ms: u128,
}

fn read_last_check() -> Option<LastCheck> {
    let body = fs::read_to_string(LAST_CHECK_FILE).ok()?;
    let value: serde_json::Value = serde_json::from_str(&body).ok()?;
    let tag = value
        .get("tag")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let at_ms = value
        .get("at_ms")
        .and_then(|v| v.as_u64())
        .map(|n| n as u128)
        .unwrap_or(0);
    Some(LastCheck { tag, at_ms })
}

fn write_last_check(check: &LastCheck) {
    let tag_repr = check
        .tag
        .as_ref()
        .map(|t| format!("{:?}", t))
        .unwrap_or_else(|| "null".into());
    let body = format!("{{\"tag\":{},\"at_ms\":{}}}", tag_repr, check.at_ms);
    // Best effort — losing the sidecar means the next start re-probes,
    // which is harmless.
    let _ = fs::write(LAST_CHECK_FILE, body);
}

/// Remove sibling `greycat-analyzer-*` directories that don't match
/// `keep`. Best effort: any IO error is logged but does not fail the
/// install (a stale dir wastes disk but never breaks the LSP).
fn prune_other_versions(keep: &str) {
    let dir = match fs::read_dir(".") {
        Ok(dir) => dir,
        Err(_) => return,
    };
    let keep_name = format!("greycat-analyzer-{keep}");
    for entry in dir.flatten() {
        let name = entry.file_name();
        let name_str = match name.to_str() {
            Some(s) => s,
            None => continue,
        };
        if name_str.starts_with("greycat-analyzer-") && name_str != keep_name {
            let _ = fs::remove_dir_all(entry.path());
        }
    }
}
