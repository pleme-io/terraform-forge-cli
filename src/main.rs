use std::path::PathBuf;
use std::process;

use clap::{Parser, Subcommand};
use colored::Colorize;

mod commands;
mod helpers_template;

#[derive(Parser)]
#[command(
    name = "terraform-forge",
    about = "Generate Terraform providers from OpenAPI specs",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate Terraform provider Go code from resource specs
    Generate {
        /// Path to OpenAPI spec (YAML or JSON)
        #[arg(long)]
        spec: PathBuf,

        /// Directory containing resource TOML specs
        #[arg(long)]
        resources: PathBuf,

        /// Output directory for generated Go files
        #[arg(long)]
        output: PathBuf,

        /// Path to provider.toml
        #[arg(long)]
        provider: Option<PathBuf>,
    },

    /// Auto-create resource spec TOMLs from OpenAPI analysis
    Scaffold {
        /// Path to OpenAPI spec
        #[arg(long)]
        spec: PathBuf,

        /// Operation ID pattern to match (e.g. "auth-method-*")
        #[arg(long)]
        pattern: Option<String>,

        /// Output directory for generated TOML files
        #[arg(long)]
        output: PathBuf,
    },

    /// Compare resource specs against OpenAPI spec, flag missing/changed
    Drift {
        /// Path to OpenAPI spec
        #[arg(long)]
        spec: PathBuf,

        /// Directory containing resource TOML specs
        #[arg(long)]
        resources: PathBuf,
    },

    /// Validate resource specs against OpenAPI spec
    Validate {
        /// Path to OpenAPI spec
        #[arg(long)]
        spec: PathBuf,

        /// Directory containing resource TOML specs
        #[arg(long)]
        resources: PathBuf,
    },

    /// Diff two OpenAPI spec versions
    Diff {
        /// Path to old spec
        #[arg(long)]
        old: PathBuf,

        /// Path to new spec
        #[arg(long)]
        new: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Generate {
            spec,
            resources,
            output,
            provider,
        } => commands::generate::run(&spec, &resources, &output, provider.as_deref()),
        Commands::Scaffold {
            spec,
            pattern,
            output,
        } => commands::scaffold::run(&spec, pattern.as_deref(), &output),
        Commands::Drift { spec, resources } => commands::drift::run(&spec, &resources),
        Commands::Validate { spec, resources } => commands::validate::run(&spec, &resources),
        Commands::Diff { old, new } => commands::diff::run(&old, &new),
    };

    if let Err(e) = result {
        eprintln!("{} {e}", "error:".red().bold());
        process::exit(1);
    }
}
