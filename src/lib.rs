use zed_extension_api::lsp::{Completion, CompletionKind};
use zed_extension_api::settings::LspSettings;
use zed_extension_api::*;

struct GreyCatExtension;

const DEFAULT_LEVEL: &str = "info";

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
        // look for the new analyzer first
        worktree
            .which("greycat-analyzer")
            .map(|command| {
                let lsp_settings = LspSettings::for_worktree(language_server_id.as_ref(), worktree)
                    .unwrap_or_default();
                let level = resolve_level(lsp_settings.settings.as_ref());

                Command {
                    command,
                    args: vec!["server".into()],
                    env: vec![
                        ("RUST_BACKTRACE".into(), "1".into()),
                        ("RUST_LOG".into(), build_rust_log(level)),
                    ],
                }
            })
            // fallback to the old one if not found
            .or_else(|| {
                worktree.which("greycat-lang").map(|command| Command {
                    command,
                    args: vec!["server".into(), "--stdio".into()],
                    env: vec![],
                })
            })
            .ok_or("unable to locate `greycat-analyzer` or `greycat-lang` in $PATH".into())
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
            _ => None,
        }
    }
}

register_extension!(GreyCatExtension);
