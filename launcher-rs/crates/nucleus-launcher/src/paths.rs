//! File-system conventions: the repository root, the Nucleus data directory,
//! app roots and the window shell.

use std::env;
use std::path::{Path, PathBuf};

/// Everything about disk layout the launcher needs at startup.
#[derive(Debug, Clone)]
pub struct Paths {
    /// `<repo>` — contains `apps/` and `assets/`.
    pub repo_root: PathBuf,
    /// `[<repo>/apps, %APPDATA%\Nucleus/apps]`, first root wins on id.
    pub app_roots: Vec<PathBuf>,
    /// `%APPDATA%\Nucleus\window-profiles\<app id>`.
    pub window_profiles_dir: PathBuf,
    /// `assets/nucleus-shell.exe` when built.
    pub shell_exe: Option<PathBuf>,
    /// `assets/nucleus.ico` (the shell expects an `.ico`, not `.svg`).
    pub icon_ico: Option<PathBuf>,
}

impl Paths {
    /// Resolve paths. `cli_repo` overrides automatic discovery of `<repo>`.
    pub fn resolve(cli_repo: Option<&Path>) -> Result<Paths, String> {
        let repo_root = match cli_repo {
            Some(path) => path.to_path_buf(),
            None => find_repo_root().ok_or(
                "could not locate the nucleus repository: I looked for an `apps/` \
                 directory upward from the current directory and from the executable. \
                 Run from inside the repository or pass --repo <path>.",
            )?,
        };
        if !repo_root.join("apps").is_dir() {
            return Err(format!(
                "`{}` does not look like the nucleus repository (no `apps/` directory)",
                repo_root.display()
            ));
        }
        let data_dir = nucleus_data_dir();
        let app_roots = vec![repo_root.join("apps"), data_dir.join("apps")];
        let assets = repo_root.join("assets");
        let exists = |p: &Path| p.is_file();
        let shell_exe =
            exists(&assets.join("nucleus-shell.exe")).then(|| assets.join("nucleus-shell.exe"));
        let icon_ico = exists(&assets.join("nucleus.ico")).then(|| assets.join("nucleus.ico"));
        Ok(Paths {
            repo_root,
            window_profiles_dir: data_dir.join("window-profiles"),
            app_roots,
            shell_exe,
            icon_ico,
        })
    }
}

/// `%APPDATA%\Nucleus`, falling back to `~/Nucleus` when `APPDATA` is unset.
pub fn nucleus_data_dir() -> PathBuf {
    let root = env::var("APPDATA").map(PathBuf::from).unwrap_or_else(|_| {
        env::var("USERPROFILE")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."))
    });
    root.join("Nucleus")
}

/// Walk up from the current directory, then from the executable, looking for a
/// directory containing `apps/`.
fn find_repo_root() -> Option<PathBuf> {
    let mut start = env::current_dir().ok()?;
    for _ in 0..12 {
        if start.join("apps").is_dir() {
            return Some(start);
        }
        start = start.parent()?.to_path_buf();
    }
    let mut start = env::current_exe().ok()?;
    start.pop(); // drop the exe name
    for _ in 0..12 {
        if start.join("apps").is_dir() {
            return Some(start);
        }
        if !start.pop() {
            break;
        }
    }
    None
}
