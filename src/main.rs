#[cfg(test)]
mod test;
mod utils;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use utils::{Profile, ProjectKind};

#[derive(Parser, Debug)]
#[command(
    name = "mux",
    version,
    about = "C/C++ project workflow tool powered by CMake presets"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Initialize a new C/C++ project template
    Init {
        /// Name of the project directory to create
        project_name: String,
        /// Template language
        #[arg(long, value_enum, default_value_t = CliProjectKind::C)]
        lang: CliProjectKind,
        /// Shortcut for --lang cpp
        #[arg(long)]
        cpp: bool,
    },
    /// Configure and build the project
    Build {
        /// Build release preset
        #[arg(long)]
        release: bool,
        /// Build both debug and release presets
        #[arg(long)]
        all: bool,
    },
    /// Run the main executable
    Run {
        /// Run using release preset
        #[arg(long)]
        release: bool,
    },
    /// Build and run tests
    Test {
        /// Test with release preset
        #[arg(long)]
        release: bool,
        /// Test both debug and release presets
        #[arg(long)]
        all: bool,
    },
    /// Clean build output
    Clean {
        /// Remove all build artifacts (out directory)
        #[arg(long)]
        all: bool,
        /// Remove only release preset output for current platform
        #[arg(long)]
        release: bool,
    },
    /// Add a C dependency into lib/ and wire it to CMake
    Add {
        /// Dependency spec: local path, git url, or GitHub shorthand (owner/repo or name)
        lib: String,
        /// Override cmake target name to link (default: directory name)
        #[arg(long)]
        target: Option<String>,
    },
    /// Remove a previously added C dependency from lib/ and CMake
    Remove {
        /// Dependency name (lib folder name)
        lib: String,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum, Eq, PartialEq)]
enum CliProjectKind {
    C,
    Cpp,
}

impl From<CliProjectKind> for ProjectKind {
    fn from(value: CliProjectKind) -> Self {
        match value {
            CliProjectKind::C => ProjectKind::C,
            CliProjectKind::Cpp => ProjectKind::Cpp,
        }
    }
}

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init {
            project_name,
            lang,
            cpp,
        } => {
            let kind = if cpp { ProjectKind::Cpp } else { lang.into() };
            utils::init_project(&project_name, kind)
        }
        Commands::Build { release, all } => {
            utils::ensure_project_root()?;
            let profiles = utils::resolve_profiles(release, all)?;
            for profile in profiles {
                utils::configure_and_build(profile)?;
            }
            Ok(())
        }
        Commands::Run { release } => {
            utils::ensure_project_root()?;
            let profile = if release {
                Profile::Release
            } else {
                Profile::Debug
            };
            utils::configure_and_build(profile)?;
            utils::run_executable(profile)
        }
        Commands::Test { release, all } => {
            utils::ensure_project_root()?;
            let profiles = utils::resolve_profiles(release, all)?;
            for profile in profiles {
                utils::configure_and_build(profile)?;
                utils::run_ctest(profile)?;
            }
            Ok(())
        }
        Commands::Clean { all, release } => {
            utils::ensure_project_root()?;
            utils::clean_outputs(all, release)
        }
        Commands::Add { lib, target } => utils::add_dependency(&lib, target),
        Commands::Remove { lib } => utils::remove_dependency(&lib),
    }
}
