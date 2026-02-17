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
        Ok(Command {
            command: worktree
                .which("greycat-lang")
                .ok_or("unable to find greycat-lang in $PATH")?,
            args: vec!["server".into(), "--stdio".into()],
            env: vec![],
        })
    }
}

register_extension!(GreyCatExtension);
