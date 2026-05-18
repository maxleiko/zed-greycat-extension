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
        // Only fn / method items get the rich rendering. The LSP
        // server emits the compact `(args): Ret` form in
        // `detail` (mirrored from `label_details.detail`); we
        // synthesize a complete `fn name(args): Ret` fragment so
        // Zed's tree-sitter pass recognizes it as a `fn_decl` and
        // applies function / parameter / type highlights from the
        // GreyCat `highlights.scm` query.
        //
        // We then declare display spans for the name + sig portions
        // only, hiding the synthetic `fn ` prefix from the popup row.
        let kind = completion.kind?;
        if !matches!(kind, CompletionKind::Function | CompletionKind::Method) {
            return None;
        }
        let detail = completion.detail.as_deref()?;
        if !detail.starts_with('(') {
            // Compact form is `(args): Ret`. If the LSP layer ever
            // emits a different shape, fall back to default
            // rendering instead of guessing.
            return None;
        }
        let name = &completion.label;
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
}

register_extension!(GreyCatExtension);
