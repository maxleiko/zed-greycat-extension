use std::path::PathBuf;

use zed_extension_api::*;

struct GreyCatExtension;

impl GreyCatExtension {
    fn lsp_bin_path(&self, worktree: &Worktree) -> String {
        let mut greycat_home = None;
        let mut home = None;
        for (name, value) in worktree.shell_env() {
            match name.as_str() {
                "GREYCAT_HOME" => {
                    greycat_home = Some(value);
                }
                "HOME" => {
                    home = Some(value);
                }
                _ => (),
            }
        }
        let greycat_home = match (home, greycat_home) {
            (Some(home), None) => PathBuf::from(home).join(".greycat"),
            (_, Some(greycat_home)) => PathBuf::from(greycat_home),
            (None, None) => PathBuf::from("/.greycat"),
        };
        greycat_home
            .join("bin")
            .join("greycat-lang")
            .to_string_lossy()
            .to_string()
    }
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
        _language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Command> {
        Ok(Command {
            command: self.lsp_bin_path(worktree),
            args: vec!["server".into(), "--stdio".into()],
            env: vec![],
        })
    }
}

register_extension!(GreyCatExtension);
