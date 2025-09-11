use std::{env::home_dir, path::PathBuf};

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
        let greycat_dir = std::env::var("GREYCAT_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let mut home_dir = home_dir().unwrap_or_else(|| "/".into());
                home_dir.push(".greycat");
                home_dir
            });
        let lsp_server = greycat_dir.join("bin").join("greycat-lang");
        Ok(Command {
            command: lsp_server.to_string_lossy().to_string(),
            args: vec!["server".into(), "--stdio".into()],
            env: vec![],
        })
    }
}

register_extension!(GreyCatExtension);
