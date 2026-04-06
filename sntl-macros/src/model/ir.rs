use darling::{FromDeriveInput, FromField};
use syn::{Ident, Type};

/// Parsed struct-level attributes from `#[sentinel(...)]`.
#[derive(Debug, FromDeriveInput)]
#[darling(attributes(sentinel), supports(struct_named))]
pub struct ModelOpts {
    pub ident: Ident,
    pub data: darling::ast::Data<(), FieldOpts>,

    /// Table name override. If None, inferred from struct name.
    #[darling(default)]
    pub table: Option<String>,
}

/// Parsed field-level attributes from `#[sentinel(...)]`.
#[derive(Debug, FromField)]
#[darling(attributes(sentinel))]
pub struct FieldOpts {
    pub ident: Option<Ident>,
    pub ty: Type,

    /// Marks this field as the primary key.
    #[darling(default)]
    pub primary_key: bool,

    /// SQL default expression (e.g., "now()"). Field will be skipped in NewModel.
    #[darling(default)]
    pub default: Option<String>,

    /// Column name override if different from field name.
    #[darling(default)]
    pub column: Option<String>,

    /// Marks column as unique (metadata for migrations, used in Phase 6).
    #[darling(default)]
    #[allow(dead_code)]
    pub unique: bool,

    /// Skip this field entirely (not a DB column).
    #[darling(default)]
    pub skip: bool,
}

/// Processed intermediate representation for code generation.
#[derive(Debug)]
pub struct ModelIR {
    pub struct_name: Ident,
    pub table_name: String,
    pub fields: Vec<FieldIR>,
    pub primary_key_index: usize,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct FieldIR {
    pub field_name: Ident,
    pub column_name: String,
    pub rust_type: Type,
    pub column_type: &'static str,
    pub nullable: bool,
    pub has_default: bool,
    pub is_primary_key: bool,
    pub skip: bool,
}

impl ModelOpts {
    /// Convert parsed darling opts into the codegen IR.
    pub fn into_ir(self) -> Result<ModelIR, darling::Error> {
        let struct_name = self.ident;

        // Infer table name: snake_case + pluralize (simple "s" suffix)
        let table_name = self.table.unwrap_or_else(|| {
            let snake = to_snake_case(&struct_name.to_string());
            format!("{snake}s")
        });

        let fields_data = self
            .data
            .take_struct()
            .expect("darling supports(struct_named) ensures this");

        let mut fields = Vec::new();
        let mut pk_index = None;

        for (i, f) in fields_data.fields.into_iter().enumerate() {
            if f.skip {
                fields.push(FieldIR {
                    field_name: f.ident.clone().unwrap(),
                    column_name: String::new(),
                    rust_type: f.ty.clone(),
                    column_type: "",
                    nullable: false,
                    has_default: false,
                    is_primary_key: false,
                    skip: true,
                });
                continue;
            }

            let field_name = f.ident.clone().unwrap();
            let column_name = f.column.unwrap_or_else(|| field_name.to_string());
            let nullable = is_option_type(&f.ty);
            let column_type = rust_type_to_column_type(&f.ty);
            let has_default = f.default.is_some();

            if f.primary_key {
                if pk_index.is_some() {
                    return Err(darling::Error::custom(
                        "only one field can be marked #[sentinel(primary_key)]",
                    )
                    .with_span(&field_name));
                }
                pk_index = Some(i);
            }

            fields.push(FieldIR {
                field_name,
                column_name,
                rust_type: f.ty,
                column_type,
                nullable,
                has_default,
                is_primary_key: f.primary_key,
                skip: false,
            });
        }

        let primary_key_index = pk_index.ok_or_else(|| {
            darling::Error::custom(
                "no field marked with #[sentinel(primary_key)] — add it to exactly one field",
            )
            .with_span(&struct_name)
        })?;

        Ok(ModelIR {
            struct_name,
            table_name,
            fields,
            primary_key_index,
        })
    }
}

/// Convert `CamelCase` to `snake_case`.
fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            result.push('_');
        }
        result.push(ch.to_ascii_lowercase());
    }
    result
}

/// Check if a type is `Option<T>`.
fn is_option_type(ty: &Type) -> bool {
    if let Type::Path(tp) = ty
        && let Some(seg) = tp.path.segments.last()
    {
        return seg.ident == "Option";
    }
    false
}

/// Map Rust types to PostgreSQL column type strings.
fn rust_type_to_column_type(ty: &Type) -> &'static str {
    let type_str = extract_type_name(ty);
    match type_str.as_str() {
        "String" => "text",
        "i32" => "int4",
        "i64" => "int8",
        "f64" => "float8",
        "bool" => "bool",
        "Uuid" => "uuid",
        "DateTime" => "timestamptz",
        "Vec" => "bytea", // Vec<u8>
        _ => "text",      // fallback
    }
}

/// Extract the outermost type name, unwrapping Option<T> if present.
fn extract_type_name(ty: &Type) -> String {
    if let Type::Path(tp) = ty
        && let Some(seg) = tp.path.segments.last()
    {
        if seg.ident == "Option"
            && let syn::PathArguments::AngleBracketed(args) = &seg.arguments
            && let Some(syn::GenericArgument::Type(inner)) = args.args.first()
        {
            return extract_type_name(inner);
        }
        return seg.ident.to_string();
    }
    "unknown".to_string()
}
