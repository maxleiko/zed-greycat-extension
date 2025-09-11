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
        _worktree: &Worktree,
    ) -> Result<Command> {
        Ok(Command {
            command: "/home/leiko/.greycat/bin/greycat-lang".into(),
            args: vec!["server".into(), "--stdio".into()],
            env: vec![],
        })
    }
}

register_extension!(GreyCatExtension);
