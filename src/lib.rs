use std::path::PathBuf;

use zed_extension_api::*;

struct GreyCatExtension;

impl GreyCatExtension {
    fn greycat_dir(worktree: &Worktree) -> PathBuf {
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
        match (home, greycat_home) {
            (_, Some(greycat_home)) => PathBuf::from(greycat_home),
            (Some(home), None) => PathBuf::from(home).join(".greycat"),
            (None, None) => PathBuf::from("/.greycat"),
        }
    }

    fn lsp_bin_path(worktree: &Worktree) -> Result<String> {
        let greycat_dir = Self::greycat_dir(worktree);
        let lsp = greycat_dir.join("bin/greycat-lang");
        if lsp.exists() {
            return Ok(lsp.to_string_lossy().to_string());
        }

        Err(format!(
            "unable to find bin/greycat-lang in {}",
            greycat_dir.display()
        ))
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
            command: Self::lsp_bin_path(worktree)?,
            args: vec!["server".into(), "--stdio".into()],
            env: vec![],
        })
    }
}

register_extension!(GreyCatExtension);
