mod core;
mod storage;
mod query;
mod server;
mod cli;
mod auth;

use clap::Parser;
use cli::{Cli, Commands, StartArgs};
use core::config::ConfigManager;
use logly::prelude::*;
use std::io::{self, Write};

#[tokio::main]
async fn main() {
    if let Err(e) = run_app().await {
        eprintln!("Error: {}", e);
        eprintln!("If you believe this is a bug, please report it at: https://github.com/muhammad-fiaz/RDB/issues");
        std::process::exit(1);
    }
}

async fn run_app() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Initialize Logly first
    let logger = std::sync::Arc::new(Logger::new());
    let mut logger_config = LoggerConfig::default();
    logger_config.color = true;
    logger.configure(logger_config);

    let mut config_manager = ConfigManager::new()?;
    let config = config_manager.load_config()?;

    // File sink
    let log_path = config_manager.root_dir.join("log").join("engine.log");
    logger.add_sink(SinkConfig {
        path: Some(log_path),
        rotation: Some("daily".to_string()),
        retention: Some(7),
        async_write: true,
        ..Default::default()
    })?;

    match &cli.command {
        Some(Commands::Start(args)) => {
            if args.silent {
                logger.remove_all_sinks();
                let log_path = config_manager.root_dir.join("log").join("engine.log");
                logger.add_sink(SinkConfig {
                    path: Some(log_path),
                    rotation: Some("daily".to_string()),
                    retention: Some(7),
                    async_write: true,
                    ..Default::default()
                })?;
            }
            start_server(args, config, &config_manager, logger.clone()).await?;
        }
        Some(Commands::Init) => {
             run_init(&mut config_manager, &logger)?;
        }
        Some(Commands::Status) => {
             print_status(&config_manager)?;
        }
        Some(Commands::Db(args)) => {
             handle_db_command(args, &config_manager, &logger)?;
        }
        Some(Commands::User(args)) => {
             handle_user_command(args, &config_manager)?;
        }
        Some(Commands::Access(args)) => {
             handle_access_command(args, &config_manager)?;
        }
        None => {
            // Default to start
            start_server(&StartArgs { listen: None, silent: false }, config, &config_manager, logger.clone()).await?;
        }
        _ => {
            println!("Command not implemented yet");
        }
    }

    Ok(())
}

fn run_init(manager: &mut ConfigManager, logger: &Logger) -> anyhow::Result<()> {
    logger.info("Initializing RDB...".to_string())?;
    
    // Check if already initialized
    let db_dir = manager.root_dir.join("databases");
    let is_first_time = !db_dir.exists() || std::fs::read_dir(&db_dir)?.next().is_none();
    
    manager.init()?;
    
    // Check for existing databases
    if db_dir.exists() {
        let existing_dbs: Vec<_> = std::fs::read_dir(&db_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("db"))
            .collect();
        
        if !existing_dbs.is_empty() {
            logger.info(format!("Found {} existing database(s):", existing_dbs.len()))?;
            for db in &existing_dbs {
                let name = db.file_name();
                logger.info(format!("  - {:?}", name))?;
            }
        }
    }
    
    // Interactive part could go here, for now just defaults
    let config = core::config::Config::default();
    manager.save_config(&config)?;
    
    // Create main database if doesn't exist
    let main_db_path = manager.get_database_path("main");
    if !main_db_path.exists() {
        logger.info("Creating 'main' database...".to_string())?;
        let pager = storage::pager::Pager::open(&main_db_path)?;
        
        // Allocate Page 0 for Header
        let header_page_id = pager.allocate_page()?;
        assert_eq!(header_page_id, 0);
        
        let header = storage::header::DatabaseHeader::new("main".to_string());
        pager.write_header(&header)?;
        
        // Create Catalog Page (Page 1)
        let page_id = pager.allocate_page()?;
        assert_eq!(page_id, 1);
        let catalog = storage::catalog::Catalog::new();
        let bytes = catalog.to_bytes()?;
        let mut page = storage::page::Page::new(page_id);
        page.data[..bytes.len()].copy_from_slice(&bytes);
        pager.write_page(&page)?;
        
        logger.success("Created database: main".to_string())?;
    }

    // First-time setup: prompt for admin user
    if is_first_time {
        logger.info("\n═══════════════════════════════════════════".to_string())?;
        logger.info("  FIRST-TIME SETUP".to_string())?;
        logger.info("═══════════════════════════════════════════\n".to_string())?;
        
        logger.info("RDB requires at least one admin user.".to_string())?;
        logger.info("You can create users later with: rdb user add <username>\n".to_string())?;
        
        print!("Would you like to create an admin user now? (y/n): ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        if input.trim().eq_ignore_ascii_case("y") {
            print!("Enter username: ");
            io::stdout().flush()?;
            let mut username = String::new();
            io::stdin().read_line(&mut username)?;
            let username = username.trim().to_string();
            
            if !username.is_empty() {
                // In real implementation, would hash password and store
                logger.success(format!("User '{}' will be created on first start.", username))?;
                logger.info("Set password with: rdb user add {} --password".to_string())?;
            }
        }
    }

    logger.info("\n✓ Initialization complete!".to_string())?;
    logger.info("  Run 'rdb start' to launch the server".to_string())?;
    logger.info("  Run 'rdb --help' for more commands\n".to_string())?;
    Ok(())
}

fn print_status(manager: &ConfigManager) -> anyhow::Result<()> {
    println!("╔════════════════════════════════════════════════════════╗");
    println!("║              RDB DATABASE STATUS                      ║");
    println!("╚════════════════════════════════════════════════════════╝\n");
    
    println!("Version: {}", env!("CARGO_PKG_VERSION"));
    println!("Config Path: {:?}", manager.config_path());
    println!("Root Directory: {:?}", manager.root_dir);
    
    // Load and display config
    match manager.load_config() {
        Ok(config) => {
            println!("\n📊 Configuration:");
            println!("  Server: {}:{}", config.server.host, config.server.port);
            println!("  Buffer Pool: {} pages ({} MB)", 
                config.storage.buffer_pool_size,
                (config.storage.buffer_pool_size * config.storage.page_size) / 1024 / 1024);
            println!("  Query Cache: {} entries ({})", 
                config.cache.query_cache_size,
                if config.cache.enable_query_cache { "enabled" } else { "disabled" });
        }
        Err(e) => {
            println!("\n⚠ Configuration: Error loading config - {}", e);
        }
    }
    
    // Discover databases
    let db_dir = manager.root_dir.join("databases");
    println!("\n💾 Databases:");
    
    if db_dir.exists() {
        let mut databases: Vec<_> = std::fs::read_dir(&db_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("db"))
            .collect();
        
        databases.sort_by_key(|e| e.file_name());
        
        if databases.is_empty() {
            println!("  (No databases found)");
            println!("  Run 'rdb init' to create the default 'main' database");
        } else {
            for db in &databases {
                let path = db.path();
                let name = path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown");
                let metadata = std::fs::metadata(&path)?;
                let size_kb = metadata.len() / 1024;
                
                println!("  • {} ({} KB)", name, size_kb);
                println!("    Path: {:?}", path);
            }
            
            println!("\n  Total: {} database(s)", databases.len());
        }
    } else {
        println!("  Database directory not found.");
        println!("  Run 'rdb init' to initialize RDB");
    }
    
    println!("\n═══════════════════════════════════════════════════════");
    println!("Run 'rdb --help' for available commands");
    Ok(())
}

fn handle_db_command(args: &cli::DbArgs, manager: &ConfigManager, logger: &Logger) -> anyhow::Result<()> {
    match &args.command {
        cli::DbCommands::Create { name } => {
            let path = manager.get_database_path(name);
            if path.exists() {
                logger.error(format!("Database {} already exists", name))?;
                return Ok(());
            }
            
            let pager = storage::pager::Pager::open(&path)?;
            
            // Allocate Page 0 for Header
            let header_page_id = pager.allocate_page()?;
            assert_eq!(header_page_id, 0);

            let header = storage::header::DatabaseHeader::new(name.clone());
            pager.write_header(&header)?;
            
            // Create Catalog Page (Page 1)
            let page_id = pager.allocate_page()?;
            assert_eq!(page_id, 1);
            let catalog = storage::catalog::Catalog::new();
            let bytes = catalog.to_bytes()?;
            let mut page = storage::page::Page::new(page_id);
            page.data[..bytes.len()].copy_from_slice(&bytes);
            pager.write_page(&page)?;

            logger.success(format!("Created database: {}", name))?;
        }
        cli::DbCommands::List => {
             print_status(manager)?;
        }
    }
    Ok(())
}

fn handle_user_command(args: &cli::UserArgs, manager: &ConfigManager) -> anyhow::Result<()> {
    let auth_manager = auth::AuthManager::new();
    let access_path = manager.root_dir.join("access_control.toml");
    if access_path.exists() {
        auth_manager.load(&access_path)?;
    }

    match &args.command {
        cli::UserCommands::Add { username, email, admin, database } => {
            // Prompt for password
            print!("Password: ");
            io::stdout().flush()?;
            let mut password = String::new();
            io::stdin().read_line(&mut password)?;
            let password = password.trim();
            
            auth_manager.add_user(username, email, password)?;
            
            // Add ACL entry if requested
            if *admin {
                println!("User {} added.", username);
            } else if let Some(_db) = database {
                 println!("User {} added. ACL modification not yet implemented via CLI.", username);
            } else {
                 println!("User {} added.", username);
            }
            
            auth_manager.save(&access_path)?;
        }
        cli::UserCommands::List => {
             println!("Listing users not implemented yet (check access_control.toml)");
        }
    }
    Ok(())
}

fn handle_access_command(args: &cli::AccessArgs, _manager: &ConfigManager) -> anyhow::Result<()> {
    match &args.command {
        cli::AccessCommands::List => {
             println!("Listing access not implemented yet (check access_control.toml)");
        }
    }
    Ok(())
}

async fn start_server(args: &StartArgs, mut config: core::config::Config, config_manager: &ConfigManager, logger: std::sync::Arc<Logger>) -> anyhow::Result<()> {
    if let Some(addr) = &args.listen {
        config.server.host = addr.clone();
    }
    
    // Print banner
    if !args.silent {
        logger.info(format!("RDB Database Engine v{}", env!("CARGO_PKG_VERSION")))?;
    }

    // Initialize Storage Engine
    let buffer_pool = std::sync::Arc::new(storage::buffer::BufferPool::new(config.storage.buffer_pool_size));
    
    // Open existing databases
    let db_dir = config_manager.root_dir.join("databases");
    if db_dir.exists() {
        for entry in std::fs::read_dir(db_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "rdb") {
                let name = path.file_stem().unwrap().to_string_lossy();
                let pager = std::sync::Arc::new(storage::pager::Pager::open(&path)?);
                
                // Register with ID. "main" -> 0. Others -> hash.
                let db_id = if name == "main" {
                    0
                } else {
                    let mut hasher = std::collections::hash_map::DefaultHasher::new();
                    use std::hash::{Hash, Hasher};
                    name.hash(&mut hasher);
                    hasher.finish() as u32
                };
                
                buffer_pool.register_pager(db_id, pager);
                if !args.silent {
                    logger.info(format!("Loaded database: {} (ID: {})", name, db_id))?;
                }
            }
        }
    }

    let executor = std::sync::Arc::new(query::executor::Executor::new(buffer_pool));
    
    // Initialize Auth
    let auth_manager = std::sync::Arc::new(auth::AuthManager::new());
    let access_path = config_manager.root_dir.join("access_control.toml");
    if access_path.exists() {
        auth_manager.load(&access_path)?;
    } else {
        // Create default admin if not exists?
    }
    
    server::run_server(config, executor, auth_manager, logger).await?;
    Ok(())
}

