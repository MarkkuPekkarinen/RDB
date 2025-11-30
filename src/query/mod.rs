use serde::{Deserialize, Serialize};

pub mod executor;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Query {
    CreateTable(CreateTableQuery),
    Insert(InsertQuery),
    Select(SelectQuery),
    Update(UpdateQuery),
    Delete(DeleteQuery),
    DropTable(DropTableQuery),
    Batch(Vec<Query>),
}

impl Query {
    pub fn get_database_name(&self) -> &str {
        match self {
            Query::CreateTable(q) => &q.database,
            Query::DropTable(q) => &q.database,
            Query::Insert(q) => &q.database,
            Query::Select(q) => &q.database,
            Query::Update(q) => &q.database,
            Query::Delete(q) => &q.database,
            Query::Batch(queries) => {
                if let Some(first) = queries.first() {
                    first.get_database_name()
                } else {
                    "unknown"
                }
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateTableQuery {
    pub database: String,
    pub table: String,
    pub columns: Vec<ColumnDef>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DropTableQuery {
    pub database: String,
    pub table: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ColumnDef {
    pub name: String,
    #[serde(rename = "type")]
    pub col_type: String,
    #[serde(default)]
    pub primary_key: bool,
    #[serde(default)]
    pub unique: bool,
    #[serde(default)]
    pub nullable: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InsertQuery {
    pub database: String,
    pub table: String,
    pub values: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SelectQuery {
    pub database: String,
    pub from: String,
    pub columns: Vec<String>,
    pub r#where: Option<WhereClause>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub join: Option<JoinClause>,
    pub order_by: Option<OrderByClause>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateQuery {
    pub database: String,
    pub table: String,
    pub set: std::collections::HashMap<String, serde_json::Value>,
    pub r#where: Option<WhereClause>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteQuery {
    pub database: String,
    pub table: String,
    pub r#where: Option<WhereClause>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WhereClause {
    pub column: String,
    pub cmp: String, // =, !=, >, <, >=, <=, LIKE, IN, BETWEEN
    pub value: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JoinClause {
    pub table: String,
    pub on: String, // "table1.col = table2.col"
    pub join_type: String, // "INNER", "LEFT"
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OrderByClause {
    pub column: String,
    pub direction: String, // "ASC", "DESC"
}
