use clap::Parser;
use dialoguer::{Completion, Confirm, Input, Select};
use directories::ProjectDirs;
use gix::discover;
use serde::{Deserialize, Serialize};
use serde_json::from_slice;
use std::collections::HashMap;
use std::env;
use std::env::home_dir;
use std::error::Error;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::exit;
use std::process::Command;

/// A command-line tool to search, add, and manage NixOS or Home Manager packages with optional automatic rebuilds.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Set config file (path to your NixOS configuration file or directory)
    #[arg(short = 'c', long = "config", value_name = "FILE")]
    config: Option<PathBuf>,

    /// Package name to add (used as search query in interactive mode or as the
    /// literal package name in --no-interactive mode)
    #[arg(short = 'p', long = "package", value_name = "PACKAGE")]
    package: Option<String>,

    /// Do not prompt interactively; fail if necessary information is missing
    #[arg(long = "no-interactive")]
    no_interactive: bool,

    /// Don't perform rebuild even if config requests it
    #[arg(long = "no-rebuild")]
    no_rebuild: bool,

    /// Remove package from NixOS config
    #[arg(short = 'r', long = "remove")]
    remove: bool,
    /// List currently configured packages
    #[arg(short = 'l', long = "list")]
    list: bool,
}

#[derive(Serialize, Deserialize, Debug)]
struct Config {
    nix_path: String,
    auto_rebuild: bool,
    home_manager: bool,
    flake: bool,
}

#[derive(Default)]
struct FileCompletion;

impl Completion for FileCompletion {
    fn get(&self, input: &str) -> Option<String> {
        // 1) First, expand possible tilde at the beginning.
        let expanded = match expand_tilde(input) {
            Ok(p) => p,
            Err(_) => return None, // if error expanding – no suggestion
        };
        // 2) Work with str representation of expanded path
        let expanded_str = expanded.to_string_lossy();
        // 3) Split into "directory" and "prefix" (filename to complete)
        let (dir_str, prefix) = match expanded_str.rfind('/') {
            Some(pos) => {
                let (d, p) = expanded_str.split_at(pos + 1); // d includes the separator
                (d.to_string(), p.to_string())
            }
            None => ("".to_string(), expanded_str.to_string()),
        };
        // 4) Select directory to search in ("." if no / in path)
        let dir_path = if dir_str.is_empty() {
            Path::new(".")
        } else {
            Path::new(&dir_str)
        };
        let read = fs::read_dir(dir_path).ok()?; // stop if cannot open
                                                 // 5) Find first file/folder whose name starts with prefix
        for entry in read.filter_map(Result::ok) {
            let name = entry.file_name();
            let name_s = name.to_string_lossy();
            if name_s.starts_with(&prefix) {
                // 6) Build completion string: dir + name (and add / if it is a directory)
                let mut completed = String::new();
                if !dir_str.is_empty() {
                    completed.push_str(&dir_str);
                }
                completed.push_str(&name_s);
                if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    completed.push('/');
                }
                return Some(completed);
            }
        }
        None
    }
}

/// Expand leading "~" in a path (if present).
fn expand_tilde(path: &str) -> Result<PathBuf, Box<dyn Error>> {
    if path.starts_with("~/") {
        let home_dir = home_dir().ok_or("Failed to get home directory")?;
        let rest_of_path = path.trim_start_matches("~/");
        let expanded_path = home_dir.join(rest_of_path);
        Ok(expanded_path)
    } else {
        Ok(PathBuf::from(path))
    }
}

fn get_git_repo_or_parent_directory(path: &PathBuf) -> Result<PathBuf, Box<dyn Error>> {
    // Check if path exists
    if !path.exists() {
        return Err("Path does not exist".into());
    }
    // Try to find a repository
    match discover(path) {
        Ok(repo) => {
            // Found repository - return its working directory
            let workdir = repo
                .workdir()
                .ok_or("Repository has no working directory")?;
            Ok(workdir.to_path_buf())
        }
        Err(_) => {
            // Repository not found
            if path.is_dir() {
                // Return the path itself (directory)
                Ok(path.to_path_buf())
            } else {
                // It's a file - return its parent directory
                let parent = path.parent().ok_or("Cannot get parent directory")?;
                Ok(parent.to_path_buf())
            }
        }
    }
}

/// If given path is a directory, try to find a likely NixOS config file inside it.
/// Returns an error if nothing suitable is found.
fn resolve_nix_config(path: &Path) -> Result<PathBuf, String> {
    if path.exists() && path.is_file() {
        return Ok(path.to_path_buf());
    } else if path.exists() && path.is_dir() {
        let candidates = [
            "configuration.nix",
            "flake.nix",
            "default.nix",
            "home.nix",
            "pkgs.nix",
        ];
        for cand in &candidates {
            let p = path.join(cand);
            if p.exists() && p.is_file() {
                return Ok(p);
            }
        }
        return Err(format!(
            "The specified directory `{}` does not contain any of the expected files: {}",
            path.display(),
            candidates.join(", ")
        ));
    }
    Err(format!("File or directory `{}` not found.", path.display()))
}

fn get_config_dir() -> Option<PathBuf> {
    let proj_dirs = ProjectDirs::from("com", "timasoft", "declair")?;
    Some(proj_dirs.config_dir().to_path_buf())
}

/// Read existing program config or interactively create it.
/// Respects `--no-interactive` from Args.
fn read_or_create_config(args: &Args) -> Result<Config, Box<dyn Error>> {
    let config_dir = get_config_dir().ok_or("Failed to get config directory")?;
    let config_path = config_dir.join("config.toml");
    if config_path.exists() {
        let contents = fs::read_to_string(&config_path)?;
        let cfg: Config = toml::from_str(&contents)?;
        Ok(cfg)
    } else {
        if args.no_interactive {
            return Err("Config file not found and --no-interactive specified".into());
        }
        fs::create_dir_all(&config_dir)?;
        let completion = FileCompletion;
        let nix_path: String = Input::new()
            .with_prompt("Enter the path to your NixOS configuration file (with 'with pkgs; [')")
            .completion_with(&completion)
            .interact_text()?;
        let auto_rebuild: bool = Confirm::new()
            .with_prompt("Automatically rebuild NixOS after adding a package?")
            .default(false)
            .interact()?;
        let (home_manager, flake) = if auto_rebuild {
            (
                Confirm::new()
                    .with_prompt("Use Home Manager as a NixOS configuration?")
                    .default(false)
                    .interact()?,
                Confirm::new()
                    .with_prompt("Use a flake as a NixOS configuration?")
                    .default(false)
                    .interact()?,
            )
        } else {
            (false, false)
        };
        let cfg = Config {
            nix_path,
            auto_rebuild,
            home_manager,
            flake,
        };
        fs::write(&config_path, toml::to_string(&cfg)?)?;
        Ok(cfg)
    }
}

#[derive(Deserialize)]
struct PackageInfo {
    pname: String,
    version: String,
    description: Option<String>,
}

/// Search for a package via `nix search`
fn search_packages(query: &str) -> Result<HashMap<String, PackageInfo>, String> {
    let output = Command::new("nix")
        .args([
            "search",
            "nixpkgs",
            query,
            "--json",
            "--extra-experimental-features",
            "nix-command flakes",
        ])
        .output()
        .map_err(|e| format!("Failed to run `nix search`: {}", e))?;
    if !output.status.success() {
        return Err("Error while running `nix search` (non-zero exit code)".to_string());
    }
    from_slice(&output.stdout).map_err(|e| format!("JSON parsing error: {}", e))
}

/// Add a package to NixOS config (input — already valid file path)
fn add_package_to_nix(file_path: &Path, pkg: &str) -> Result<(), Box<dyn Error>> {
    let file = fs::File::open(file_path)?;
    let reader = BufReader::new(file);
    let mut lines: Vec<String> = reader.lines().collect::<Result<_, _>>()?;
    // make backup (overwrite if already exists)
    fs::copy(file_path, file_path.with_extension("declair.bak"))?;
    // find start and end of "with pkgs; [" block
    if let Some(start_idx) = lines
        .iter()
        .position(|l: &String| l.contains("with pkgs; ["))
        && let Some(end_idx_rel) = lines[start_idx..]
            .iter()
            .position(|l: &String| l.contains(']'))
    {
        let end_idx = start_idx + end_idx_rel;
        // find line with pkg
        for line in lines[start_idx..end_idx].iter() {
            if line.contains(pkg) {
                return Err(format!("Package `{}` is already in the config", pkg).into());
            }
        }
        // clone the line and indentation BEFORE mutations, to avoid borrow issues
        let end_line = lines[end_idx].clone();
        // three cases (simplified but reliable logic)
        if start_idx == end_idx {
            // everything in one line, e.g.: with pkgs; []
            if end_line.contains("[]") {
                lines[start_idx] = end_line.replace("[]", &format!("[ {} ]", pkg));
            } else if end_line.contains(" ]") {
                lines[start_idx] = end_line.replace("]", &format!("{} ]", pkg));
            } else {
                lines[start_idx] = end_line.replace("]", &format!(" {} ]", pkg));
            }
        } else {
            // multiline case
            let indent: String = end_line.chars().take_while(|c| c.is_whitespace()).collect();
            lines.insert(end_idx, format!("{}{}{}", indent, indent, pkg));
        }
    } else {
        return Err("Failed to find `with pkgs; [...]` block in the given file.".into());
    }
    fs::write(file_path, lines.join("\n"))?;
    Ok(())
}

/// List packages found in `with pkgs; [ ... ]` block of given file.
fn list_packages(file_path: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    let file = fs::File::open(file_path)?;
    let reader = BufReader::new(file);
    let lines: Vec<String> = reader.lines().collect::<Result<_, _>>()?;

    if let Some(start_idx) = lines
        .iter()
        .position(|l: &String| l.contains("with pkgs; ["))
        && let Some(end_idx_rel) = lines[start_idx..]
            .iter()
            .position(|l: &String| l.contains(']'))
    {
        let end_idx = start_idx + end_idx_rel;
        let mut packages: Vec<String> = Vec::new();

        if start_idx == end_idx {
            // single-line case
            let line = &lines[start_idx];
            if let Some(lbr) = line.find('[')
                && let Some(rbr) = line.rfind(']')
            {
                let inside = &line[lbr + 1..rbr];
                for token in inside.split_whitespace() {
                    if !token.trim().is_empty() {
                        packages.push(token.trim().to_string());
                    }
                }
            }
        } else {
            // multiline case: lines between start_idx+1 .. end_idx-1
            for l in &lines[start_idx + 1..end_idx] {
                let trimmed = l.trim();
                if trimmed.is_empty() {
                    continue;
                }
                // take the first token on the line as package name
                if let Some(tok) = trimmed.split_whitespace().next() {
                    // skip lines that are just comments
                    if tok.starts_with('#') || tok.starts_with("//") {
                        continue;
                    }
                    packages.push(tok.to_string());
                }
            }
        }
        Ok(packages)
    } else {
        Err("Failed to find `with pkgs; [...]` block in the given file.".into())
    }
}

/// Remove a package from NixOS config (with backup). Does not perform rebuild itself.
fn remove_package_from_nix(file_path: &Path, pkg: &str) -> Result<(), Box<dyn Error>> {
    let file = fs::File::open(file_path)?;
    let reader = BufReader::new(&file);
    let mut lines: Vec<String> = reader.lines().collect::<Result<_, _>>()?;

    // make backup (overwrite if already exists)
    fs::copy(file_path, file_path.with_extension("declair.bak"))?;

    // find start and end of "with pkgs; [" block
    if let Some(start_idx) = lines.iter().position(|l| l.contains("with pkgs; ["))
        && let Some(end_idx_rel) = lines[start_idx..]
            .iter()
            .position(|l: &String| l.contains(']'))
    {
        let end_idx = start_idx + end_idx_rel;

        if start_idx == end_idx {
            // single-line case
            let line = &lines[start_idx];
            let lbr = line
                .find('[')
                .ok_or("Malformed `with pkgs; [ ... ]` line")?;
            let rbr = line
                .rfind(']')
                .ok_or("Malformed `with pkgs; [ ... ]` line")?;
            let inside = &line[lbr + 1..rbr];
            let parts: Vec<&str> = inside
                .split_whitespace()
                .filter(|s| !s.is_empty())
                .collect();
            if !parts.contains(&pkg) {
                return Err(format!("Package `{}` not found in the configuration", pkg).into());
            }
            let new_parts: Vec<&str> = parts.into_iter().filter(|&p| p != pkg).collect();
            let new_inside = new_parts.join(" ");
            let new_line = format!("{}[ {} ]", &line[..lbr], new_inside);
            lines[start_idx] = new_line;
        } else {
            // multiline case
            // find the index of the line that contains the package (first token matches)
            let mut found_idx: Option<usize> = None;
            for (i, l) in lines[start_idx + 1..end_idx].iter().enumerate() {
                let trimmed = l.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if let Some(first) = trimmed.split_whitespace().next()
                    && first == pkg
                {
                    found_idx = Some(start_idx + 1 + i);
                    break;
                }
            }
            if found_idx.is_none() {
                return Err(format!("Package `{}` not found in the configuration", pkg).into());
            }
            let remove_idx = found_idx.unwrap();
            lines.remove(remove_idx);
        }
    } else {
        return Err("Failed to find `with pkgs; [...]` block in the given file.".into());
    }

    fs::write(file_path, lines.join("\n"))?;
    Ok(())
}

fn main() {
    let args = Args::parse();

    // top-level error handling
    if let Err(e) = run(args) {
        eprintln!("Error: {}", e);
        exit(1);
    }
}

fn run(args: Args) -> Result<(), Box<dyn Error>> {
    let mut config = read_or_create_config(&args)?;

    // If user passed --config, override the nix_path from the stored config.
    if let Some(cfg_path) = &args.config {
        config.nix_path = cfg_path.to_string_lossy().to_string();
    }

    // expand and resolve nix config path
    let raw = config.nix_path.trim();
    let expanded = expand_tilde(raw)?;
    let nix_file = resolve_nix_config(&expanded)
        .map_err(|s| format!("Failed to use path `{}`: {}", expanded.display(), s))?;
    let git_repo = get_git_repo_or_parent_directory(&nix_file)?;

    // Handle --list first: just list packages and exit
    if args.list {
        match list_packages(&nix_file) {
            Ok(pkgs) => {
                if pkgs.is_empty() {
                    println!(
                        "No packages found in `with pkgs; [...]` block of {}",
                        nix_file.display()
                    );
                } else {
                    // Заголовки колонок
                    let header_pkg = "Package";
                    let header_src = "Source";

                    // Ширина первой колонки — максимум из длин пакетов и заголовка
                    let w1 = pkgs
                        .iter()
                        .map(|s| s.len())
                        .max()
                        .unwrap_or(0)
                        .max(header_pkg.len());

                    // Вторая колонка — путь к файлу (одинаков для всех строк) или заголовок
                    let source = format!("{}", nix_file.display());
                    let w2 = source.len().max(header_src.len());

                    // Печатаем заголовок
                    println!(
                        "{:<w1$} | {:<w2$}",
                        header_pkg,
                        header_src,
                        w1 = w1,
                        w2 = w2
                    );

                    // Корректная разделительная линия — через repeat
                    println!("{}-+-{}", "-".repeat(w1), "-".repeat(w2));

                    // Печатаем строки
                    for p in pkgs {
                        println!("{:<w1$} | {:<w2$}", p, source, w1 = w1, w2 = w2);
                    }
                }
                return Ok(());
            }
            Err(e) => {
                return Err(format!("Failed to list packages: {}", e).into());
            }
        }
    }

    // obtain query: from CLI or interactively (existing add-package flow)
    let query: String = if let Some(q) = args.package.clone() {
        q
    } else if args.no_interactive {
        return Err("No query provided and --no-interactive specified".into());
    } else {
        Input::new()
            .with_prompt("Search for a package")
            .interact_text()?
    };

    let mut options = Vec::new();

    let selected_pkg = if args.no_interactive {
        query
    } else {
        let pkg_map: HashMap<String, PackageInfo> =
            search_packages(&query).map_err(|s| format!("Package search failed: {}", s))?;
        if pkg_map.is_empty() {
            println!("No results found");
            return Ok(());
        }
        for pkg in pkg_map.values() {
            let desc = pkg.description.as_deref().unwrap_or("");
            options.push(format!("{} {}: {}", pkg.pname, pkg.version, desc));
        }

        let selection = Select::new()
            .with_prompt("Select a package:")
            .items(&options)
            .default(0)
            .interact()?;
        let selected_line = &options[selection];
        // safer to extract and own the package name
        selected_line
            .split_whitespace()
            .next()
            .ok_or("Failed to extract package name")?
            .to_string()
    };

    if args.remove {
        remove_package_from_nix(&nix_file, &selected_pkg)?;
        println!("Removed `{}` to `{}`", selected_pkg, nix_file.display());
    } else {
        add_package_to_nix(&nix_file, &selected_pkg)?;
        println!("Added `{}` to `{}`", selected_pkg, nix_file.display());
    }

    // Respect --no-rebuild flag
    if config.auto_rebuild && !args.no_rebuild {
        println!("Rebuilding NixOS with the new package...");
        env::set_current_dir(&git_repo)?;
        let status = if config.flake {
            if config.home_manager {
                Command::new("home-manager")
                    .args(["switch", "--flake", "."])
                    .status()?
            } else {
                Command::new("sudo")
                    .args(["nixos-rebuild", "switch", "--flake", "."])
                    .status()?
            }
        } else if config.home_manager {
            Command::new("home-manager").args(["switch"]).status()?
        } else {
            Command::new("sudo")
                .args(["nixos-rebuild", "switch"])
                .status()?
        };
        if !status.success() {
            eprintln!("Error while running nixos-rebuild (exit code != 0)");
        }
    } else if config.auto_rebuild && args.no_rebuild {
        println!("Skipping rebuild due to --no-rebuild flag");
    }

    println!("Done");
    Ok(())
}
