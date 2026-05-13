//! `sntl` CLI — `prepare`, `check`, `doctor`.

use clap::{Parser, Subcommand};

mod commands;
mod scan;
mod ui;

#[derive(Parser)]
#[command(name = "sntl", version, about = "Sentinel ORM CLI")]
struct Cli {
    #[arg(long, global = true, help = "Workspace root (default: auto-detect)")]
    workspace: Option<std::path::PathBuf>,
    #[arg(long, global = true, help = "Override DATABASE_URL from sentinel.toml")]
    database_url: Option<String>,
    #[command(subcommand)]
    cmd: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Scaffold sentinel.toml + .sentinel/ in the workspace root
    Init {
        #[arg(long, help = "Overwrite sentinel.toml if it already exists")]
        force: bool,
    },
    /// Scan workspace and cache query metadata in .sentinel/
    Prepare {
        #[arg(long, help = "Do not write anything; exit 1 if stale")]
        check: bool,
    },
    /// Validate existing .sentinel/ cache
    Check,
    /// Diagnose config, DB, and cache health
    Doctor,
    /// Manage SQL migrations
    Migrate {
        #[command(subcommand)]
        action: MigrateCmd,
    },
}

#[derive(Subcommand)]
pub enum MigrateCmd {
    /// Scaffold a new migration folder under `migrations/`
    Add {
        /// Free-form name (will be lowercased and snake-cased)
        name: String,
        #[arg(long, help = "Fail if migrations/ does not exist instead of creating it")]
        no_create_dir: bool,
    },
    /// Apply pending migrations in version order
    Run {
        #[arg(long, help = "List pending migrations without applying")]
        dry_run: bool,
        #[arg(long, help = "Skip the post-apply refresh of .sentinel/schema.toml")]
        skip_refresh: bool,
    },
    /// Show applied + pending migration status
    Info {
        #[arg(long)]
        applied: bool,
        #[arg(long)]
        pending: bool,
        #[arg(long)]
        all: bool,
    },
    /// Compare `.sentinel/schema.toml` with the live DB and emit a SQL scaffold
    Diff {
        #[arg(long, help = "Suffix for the generated migration folder name")]
        out: Option<String>,
    },
    /// Verify applied migrations still match their on-disk files
    Verify,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Command::Init { force } => commands::init::run(cli.workspace, force),
        Command::Prepare { check } => {
            commands::prepare::run(cli.workspace, cli.database_url, check).await
        }
        Command::Check => commands::check::run(cli.workspace).await,
        Command::Doctor => commands::doctor::run(cli.workspace, cli.database_url).await,
        Command::Migrate { action } => {
            commands::migrate::dispatch(cli.workspace, cli.database_url, action).await
        }
    }
}
