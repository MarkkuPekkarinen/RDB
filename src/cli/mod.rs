use clap::{Parser, Subcommand, Args};

#[derive(Parser)]
#[command(name = "rdb")]
#[command(about = "RDB: A high-performance relational database engine", long_about = None)]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize a new RDB environment
    Init,
    /// Start the RDB server
    Start(StartArgs),
    /// Show RDB status
    Status,
    /// Database management
    Db(DbArgs),
    /// User management
    User(UserArgs),
    /// Access control management
    Access(AccessArgs),
    /// Configuration management
    Config(ConfigArgs),
    /// Interactive shell
    Shell(ShellArgs),
}

#[derive(Args)]
pub struct StartArgs {
    #[arg(long)]
    pub listen: Option<String>,
    #[arg(long)]
    pub silent: bool,
}

#[derive(Args)]
pub struct DbArgs {
    #[command(subcommand)]
    pub command: DbCommands,
}

#[derive(Subcommand)]
pub enum DbCommands {
    Create { name: String },
    List,
}

#[derive(Args)]
pub struct UserArgs {
    #[command(subcommand)]
    pub command: UserCommands,
}

#[derive(Subcommand)]
pub enum UserCommands {
    Add { 
        username: String,
        #[arg(long)]
        email: String,
        #[arg(long)]
        admin: bool,
        #[arg(long)]
        database: Option<String>
    },
    List,
}

#[derive(Args)]
pub struct AccessArgs {
    #[command(subcommand)]
    pub command: AccessCommands,
}

#[derive(Subcommand)]
pub enum AccessCommands {
    List,
}

#[derive(Args)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub command: ConfigCommands,
}

#[derive(Subcommand)]
pub enum ConfigCommands {
    /// Show current configuration
    Show,
    /// Get a specific configuration value
    Get { key: String },
    /// Set a configuration value
    Set { key: String, value: String },
    /// Reload configuration from file
    Reload,
    /// Reset configuration to defaults
    Reset,
}

#[derive(Args)]
pub struct ShellArgs {
    #[arg(long)]
    pub database: Option<String>,
}
