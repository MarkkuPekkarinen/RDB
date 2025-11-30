use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;
use argon2::{
    password_hash::{
        PasswordHash, PasswordHasher, PasswordVerifier, SaltString
    },
    Argon2,
};
use rand::RngCore;
use anyhow::{Result, anyhow};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Role {
    Owner,
    DbAdmin,
    ReadWrite,
    ReadOnly,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    pub username: String,
    pub email: String,
    pub password_hash: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AclEntry {
    pub username: String,
    pub database: String,
    pub role: Role,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct AccessControl {
    pub users: Vec<User>,
    pub access: Vec<AclEntry>,
}

pub struct AuthManager {
    access_control: RwLock<AccessControl>,
    sessions: RwLock<HashMap<String, String>>, // token -> username
}

impl AuthManager {
    pub fn new() -> Self {
        Self {
            access_control: RwLock::new(AccessControl::default()),
            sessions: RwLock::new(HashMap::new()),
        }
    }

    pub fn load(&self, path: &std::path::Path) -> Result<()> {
        if !path.exists() {
            return Ok(());
        }
        let content = std::fs::read_to_string(path)?;
        let ac: AccessControl = toml::from_str(&content)?;
        *self.access_control.write().unwrap() = ac;
        Ok(())
    }

    pub fn save(&self, path: &std::path::Path) -> Result<()> {
        let ac = self.access_control.read().unwrap();
        let content = toml::to_string_pretty(&*ac)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn add_user(&self, username: &str, email: &str, password: &str) -> Result<()> {
        let mut salt_bytes = [0u8; 16];
        rand::rng().fill_bytes(&mut salt_bytes);
        let salt = SaltString::encode_b64(&salt_bytes)
            .map_err(|e| anyhow!(e.to_string()))?;

        let argon2 = Argon2::default();
        let password_hash = argon2.hash_password(password.as_bytes(), &salt)
            .map_err(|e| anyhow!(e.to_string()))?
            .to_string();

        let mut ac = self.access_control.write().unwrap();
        if ac.users.iter().any(|u| u.username == username) {
            return Err(anyhow!("User already exists"));
        }
        
        ac.users.push(User {
            username: username.to_string(),
            email: email.to_string(),
            password_hash,
        });
        Ok(())
    }

    pub fn login(&self, username: &str, password: &str) -> Result<String> {
        let ac = self.access_control.read().unwrap();
        let user = ac.users.iter().find(|u| u.username == username)
            .ok_or(anyhow!("Invalid username or password"))?;

        let parsed_hash = PasswordHash::new(&user.password_hash)
            .map_err(|e| anyhow!(e.to_string()))?;
            
        Argon2::default().verify_password(password.as_bytes(), &parsed_hash)
            .map_err(|_| anyhow!("Invalid username or password"))?;

        // Generate token
        let token = uuid::Uuid::new_v4().to_string();

        self.sessions.write().unwrap().insert(token.clone(), username.to_string());
        Ok(token)
    }

    pub fn check_access(&self, token: &str, database: &str, required_role: Role) -> Result<()> {
        let sessions = self.sessions.read().unwrap();
        let username = sessions.get(token).ok_or(anyhow!("Invalid session token"))?;
        
        let ac = self.access_control.read().unwrap();
        
        // Check ACL
        // Simplification: Owner > DbAdmin > ReadWrite > ReadOnly
        // We need to implement role hierarchy comparison
        
        let entry = ac.access.iter().find(|e| e.username == *username && e.database == database);
        
        if let Some(entry) = entry {
            // Check role sufficiency
            if self.role_sufficient(&entry.role, &required_role) {
                return Ok(());
            }
        }
        
        // Check if user is global admin (not implemented yet, maybe "admin" database?)
        
        Err(anyhow!("Access denied"))
    }
    
    fn role_sufficient(&self, user_role: &Role, required: &Role) -> bool {
        match user_role {
            Role::Owner => true,
            Role::DbAdmin => matches!(required, Role::DbAdmin | Role::ReadWrite | Role::ReadOnly),
            Role::ReadWrite => matches!(required, Role::ReadWrite | Role::ReadOnly),
            Role::ReadOnly => matches!(required, Role::ReadOnly),
        }
    }
}
