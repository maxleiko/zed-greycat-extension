use zed_extension_api::*;

struct GreyCatExtension;

impl Extension for GreyCatExtension {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self
    }

    fn language_server_command(
        &mut self,
        _language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Command> {
        // look for the new analyzer first
        worktree
            .which("greycat-analyzer")
            .map(|command| Command {
                command,
                args: vec!["server".into()],
                env: vec![],
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
}

register_extension!(GreyCatExtension);
