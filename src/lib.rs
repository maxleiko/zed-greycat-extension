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
}

register_extension!(GreyCatExtension);
