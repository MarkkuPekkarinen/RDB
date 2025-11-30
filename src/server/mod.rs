use actix_web::{web, App, HttpServer, HttpResponse, Responder};
use crate::core::config::Config as RdbConfig;
use crate::query::executor::{Executor, ExecutionResult};
use crate::query::Query;
use crate::auth::AuthManager;
use std::sync::Arc;
use logly::Logger;

pub struct AppState {
    pub executor: Arc<Executor>,
    pub auth: Arc<AuthManager>,
    pub config: RdbConfig,
    pub logger: Arc<Logger>,
}

pub async fn run_server(config: RdbConfig, executor: Arc<Executor>, auth: Arc<AuthManager>, logger: Arc<Logger>) -> std::io::Result<()> {
    let bind_addr = format!("{}:{}", config.server.host, config.server.port);
    let _ = logger.info(format!("Starting RDB Server at http://{}", bind_addr));

    let app_state = web::Data::new(AppState {
        executor,
        auth,
        config: config.clone(),
        logger: logger.clone(),
    });

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .route("/", web::get().to(index))
            .route("/status", web::get().to(status))
            .route("/query", web::post().to(query_handler))
            .route("/login", web::post().to(login_handler))
    })
    .bind(bind_addr)?
    .run()
    .await
}

async fn index() -> impl Responder {
    HttpResponse::Ok().body("RDB Server is running")
}

async fn status() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "status": "healthy"
    }))
}

#[derive(serde::Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

async fn login_handler(
    data: web::Data<AppState>,
    req: web::Json<LoginRequest>,
) -> impl Responder {
    match data.auth.login(&req.username, &req.password) {
        Ok(token) => HttpResponse::Ok().json(serde_json::json!({ "status": "success", "token": token })),
        Err(e) => HttpResponse::Unauthorized().json(serde_json::json!({ "status": "error", "message": e.to_string() })),
    }
}

async fn query_handler(
    data: web::Data<AppState>,
    query: web::Json<Query>,
    req: actix_web::HttpRequest,
) -> impl Responder {
    // Check Auth
    if data.config.auth.enabled {
        let auth_header = req.headers().get("Authorization");
        if let Some(header_val) = auth_header {
            if let Ok(header_str) = header_val.to_str() {
                if header_str.starts_with("Bearer ") {
                    let token = &header_str[7..];
                    // Verify token and permissions
                    // We need database name from query to check ACL
                    let db_name = query.get_database_name();
                    
                    // For now, assume ReadWrite role is needed for everything
                    // In real impl, Select -> ReadOnly, others -> ReadWrite
                    let required_role = match &*query {
                        Query::Select(_) => crate::auth::Role::ReadOnly,
                        Query::Batch(queries) => {
                            // If any query is not Select, require ReadWrite
                            if queries.iter().all(|q| matches!(q, Query::Select(_))) {
                                crate::auth::Role::ReadOnly
                            } else {
                                crate::auth::Role::ReadWrite
                            }
                        },
                        _ => crate::auth::Role::ReadWrite,
                    };
                    
                    if let Err(e) = data.auth.check_access(token, db_name, required_role) {
                         return HttpResponse::Forbidden().json(serde_json::json!({ "status": "error", "message": e.to_string() }));
                    }
                } else {
                     return HttpResponse::Unauthorized().json(serde_json::json!({ "status": "error", "message": "Invalid token format" }));
                }
            } else {
                 return HttpResponse::Unauthorized().json(serde_json::json!({ "status": "error", "message": "Invalid header" }));
            }
        } else {
             return HttpResponse::Unauthorized().json(serde_json::json!({ "status": "error", "message": "Missing Authorization header" }));
        }
    }

    match data.executor.execute(query.into_inner()) {
        Ok(result) => match result {
            ExecutionResult::Message(msg) => HttpResponse::Ok().json(serde_json::json!({ "status": "success", "message": msg })),
            ExecutionResult::Json(val) => HttpResponse::Ok().json(serde_json::json!({ "status": "success", "data": val })),
        },
        Err(e) => {
            let _ = data.logger.error(format!("Query execution error: {}", e));
            HttpResponse::BadRequest().json(serde_json::json!({ "status": "error", "message": e.to_string() }))
        }
    }
}
