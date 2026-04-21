use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    pub version: u32,
    pub postgres_version: String,
    #[serde(default)]
    pub generated_at: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub tables: Vec<Table>,
    #[serde(default)]
    pub enums: Vec<EnumType>,
    #[serde(default)]
    pub composites: Vec<CompositeType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table {
    pub name: String,
    #[serde(default = "default_schema")]
    pub schema: String,
    #[serde(default)]
    pub columns: Vec<Column>,
    #[serde(default)]
    pub foreign_keys: Vec<ForeignKey>,
}

fn default_schema() -> String {
    "public".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Column {
    pub name: String,
    #[serde(flatten)]
    pub pg_type: PgTypeRef,
    pub oid: u32,
    #[serde(default)]
    pub nullable: bool,
    #[serde(default)]
    pub primary_key: bool,
    #[serde(default)]
    pub unique: bool,
    #[serde(default)]
    pub default: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PgTypeRef {
    pub pg_type: String,
}

impl PgTypeRef {
    pub fn simple(name: &str) -> Self {
        Self {
            pg_type: name.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForeignKey {
    pub columns: Vec<String>,
    pub references: FkTarget,
    #[serde(default)]
    pub on_delete: Option<String>,
    #[serde(default)]
    pub on_update: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FkTarget {
    pub table: String,
    pub columns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumType {
    pub name: String,
    pub values: Vec<String>,
    pub oid: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositeType {
    pub name: String,
    pub fields: Vec<CompositeField>,
    #[serde(default)]
    pub oid: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositeField {
    pub name: String,
    pub pg_type: String,
    #[serde(default)]
    pub nullable: bool,
}

impl Schema {
    pub fn find_table(&self, name: &str) -> Option<&Table> {
        self.tables.iter().find(|t| t.name == name)
    }
    pub fn find_column(&self, table: &str, column: &str) -> Option<&Column> {
        self.find_table(table)?
            .columns
            .iter()
            .find(|c| c.name == column)
    }
    pub fn find_enum(&self, name: &str) -> Option<&EnumType> {
        self.enums.iter().find(|e| e.name == name)
    }
}
