//! This crate allows you to permanently set environment variables
//!
//! # Examples
//! ```rust
//! // Check if DUMMY is set, if not set it to 1
//! // export DUMMY=1
//! set_env::check_or_set("DUMMY", 1).expect("Failed to find or set DUMMY");
//! // Append $HOME/some/cool/bin to $PATH
//! // export PATH= "$HOME/some/cool/bin:$PATH"
//! set_env::append("PATH", "$HOME/some/cool/bin").expect("Couldn't find PATH");
//! // Sets a variable without checking if it exists.
//! // Note you need to use a raw string literal to include ""
//! // export DUMMY="/something"
//! set_env::set("DUMMY", r#""/something""#).expect("Failed to set DUMMY");
//! ```

#[cfg(target_family = "unix")]
use dirs;
#[cfg(target_family = "unix")]
use std::fs::{File, OpenOptions};
#[cfg(target_family = "unix")]
use std::io::Write;
#[cfg(target_family = "unix")]
use std::path::PathBuf;

use std::env;
use std::env::VarError;
use std::fmt;
use std::io;

#[cfg(target_family = "windows")]
pub fn do_prerequisites() {
    use std::fs;

    let path = dirs::document_dir();
    if let Some(path) = path {
        let pkg_version = env!("CARGO_PKG_VERSION");

        let template = include_str!("../scripts/profile.ps1").replace("${VER}", pkg_version);
        let path = path.join("WindowsPowerShell");
        if !path.exists() {
            fs::create_dir_all(&path).unwrap();
        }
        let path = path.join("Profile.ps1");
        if !path.exists() {
            fs::write(&path, template).unwrap();
        } else {
            let prefix = "# ----------------------------------VER";
            let content = fs::read_to_string(&path).unwrap();

            let mut lines = content.lines();

            let pos = lines.position(|it| it.starts_with(prefix));

            if let Some(pos) = pos {
                let content = lines.nth(pos).unwrap().replace(prefix, "");
                if content != pkg_version {
                    fs::write(&path, template).unwrap();
                }
            } else {
                fs::write(&path, template).unwrap();
            }
        }
        return;
    }

    eprintln!("document path is not exists");
    std::process::exit(1);
}

#[cfg(target_os = "windows")]
pub fn inject(it: &str) -> io::Result<()> {
    use std::fs;

    do_prerequisites();

    let profile_path = dirs::document_dir()
        .unwrap()
        .join("WindowsPowerShell/Profile.ps1");

    let content = fs::read_to_string(&profile_path)?;
    let mut content_parts: Vec<&str> = content.split("\r\n").collect();

    let idx = content_parts
        .iter()
        .position(|it| it == &"# ----------------------------------SET_ENV_DEFS_END")
        .unwrap();
    content_parts.insert(idx, it);

    fs::write(profile_path, content_parts.join("\r\n"))
}

/// Checks if a environment variable is set.
/// If it is then nothing will happen.
/// If it's not then it will be added
/// to your profile.
pub fn check_or_set<T, U>(var: T, value: U) -> io::Result<()>
where
    T: fmt::Display + AsRef<std::ffi::OsStr>,
    U: fmt::Display,
{
    env::var(&var).map(|_| ()).or_else(|_| set(var, value))
}

pub fn get<T: fmt::Display>(var: T) -> io::Result<String> {
    env::var(var.to_string()).map_err(|err| match err {
        VarError::NotPresent => io::Error::new(io::ErrorKind::NotFound, "Variable not present."),
        VarError::NotUnicode(_) => {
            io::Error::new(io::ErrorKind::Unsupported, "Encoding not supported.")
        }
    })
}

/// Appends a value to an environment variable
/// Useful for appending a value to PATH
#[cfg(target_family = "unix")]
pub fn append<T: fmt::Display>(var: T, value: T) -> io::Result<()> {
    let mut profile = get_profile()?;
    writeln!(profile, "\nexport {}=\"{}:${}\"", var, value, var)?;
    profile.flush()
}
/// Appends a value to an environment variable
/// Useful for appending a value to PATH
#[cfg(target_os = "windows")]
pub fn append<T: fmt::Display>(var: T, value: T) -> io::Result<()> {
    inject(format!("setenv_append {} {}", var, value).as_str())
}

/// Prepends a value to an environment variable
/// Useful for prepending a value to PATH
#[cfg(target_family = "unix")]
pub fn prepend<T: fmt::Display>(var: T, value: T) -> io::Result<()> {
    let mut profile = get_profile()?;
    writeln!(profile, "\nexport {}=\"${}:{}\"", var, value, var)?;
    profile.flush()
}

/// Prepends a value to an environment variable
/// Useful for prepending a value to PATH
#[cfg(target_os = "windows")]
pub fn prepend<T: fmt::Display>(var: T, value: T) -> io::Result<()> {
    inject(format!("setenv_prepend {} {}", var, value).as_str())
}

/// Sets an environment variable without checking
/// if it exists.
/// If it does you will end up with two
/// assignments in your profile.
/// It's recommended to use `check_or_set`
/// unless you are certain it doesn't exist.
#[cfg(target_family = "unix")]
pub fn set<T: fmt::Display, U: fmt::Display>(var: T, value: U) -> io::Result<()> {
    let mut profile = get_profile()?;
    writeln!(profile, "\nexport {}={}", var, value)?;
    profile.flush()
}
/// Sets an environment variable without checking
/// if it exists.
/// If it does you will override the value.
#[cfg(target_os = "windows")]
pub fn set<T: fmt::Display, U: fmt::Display>(var: T, value: U) -> io::Result<()> {
    inject(format!("setenv_set {} {}", var, value).as_str())?;
    Ok(())
}

#[cfg(target_family = "unix")]
fn get_profile() -> io::Result<File> {
    let home_dir = dirs::home_dir()
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "No home directory"))?;

    let profile_path = find_profile(home_dir.clone()).unwrap_or_else(|_| {
        let mut fallback = home_dir;
        fallback.push(".profile");
        fallback
    });

    let mut oo = OpenOptions::new();
    oo.append(true).create(true);
    oo.open(profile_path)
}

struct Shell {
    name: &'static str,
    config_files: &'static [&'static str],
}

static SHELLS: &[Shell] = &[
    Shell {
        name: "zsh",
        config_files: &[".zprofile", ".zshrc", ".zlogin"],
    },
    Shell {
        name: "fish",
        config_files: &[".config/fish/config.fish"],
    },
    Shell {
        name: "tcsh",
        config_files: &[".tcshrc", ".cshrc", ".login"],
    },
    Shell {
        name: "csh",
        config_files: &[".tcshrc", ".cshrc", ".login"],
    },
    Shell {
        name: "ksh",
        config_files: &[".profile", ".kshrc"],
    },
    Shell {
        name: "bash",
        config_files: &[".bash_profile", ".bash_login", ".bashrc"],
    },
];

#[cfg(target_family = "unix")]
fn find_profile(mut home_dir: PathBuf) -> io::Result<PathBuf> {
    let shell_env = env::var("SHELL").unwrap_or_default();

    let selected_shell = SHELLS
        .iter()
        .find(|s| shell_env.contains(s.name))
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Unsupported shell"))?;

    for config_file in selected_shell.config_files {
        let mut config_path = home_dir.clone();
        for part in config_file.split('/') {
            config_path.push(part);
        }

        if config_path.exists() {
            return Ok(config_path);
        }

        if config_file.contains('/') {
            if std::fs::create_dir_all(config_path.parent().unwrap()).is_ok() {
                return Ok(config_path);
            }
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Cannot create config directory",
            ));
        }
    }

    home_dir.push(selected_shell.config_files[0]);
    Ok(home_dir)
}
