use darling::{FromDeriveInput, FromField};
use syn::{Ident, Type};

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(sentinel), supports(struct_named))]
pub struct PartialOpts {
    pub ident: Ident,
    pub data: darling::ast::Data<(), PartialFieldOpts>,

    /// The model this partial type selects from (e.g., "User").
    pub model: String,
}

#[derive(Debug, FromField)]
#[darling(attributes(sentinel))]
#[allow(dead_code)]
pub struct PartialFieldOpts {
    pub ident: Option<Ident>,
    pub ty: Type,
}

#[derive(Debug)]
pub struct PartialIR {
    pub struct_name: Ident,
    pub model_name: String,
    pub fields: Vec<PartialFieldIR>,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct PartialFieldIR {
    pub field_name: Ident,
    pub column_name: String,
}

impl PartialOpts {
    pub fn into_ir(self) -> Result<PartialIR, darling::Error> {
        let fields_data = self
            .data
            .take_struct()
            .expect("darling supports(struct_named) ensures this");

        let fields = fields_data
            .fields
            .into_iter()
            .map(|f| {
                let field_name = f.ident.clone().unwrap();
                let column_name = field_name.to_string();
                PartialFieldIR {
                    field_name,
                    column_name,
                }
            })
            .collect();

        Ok(PartialIR {
            struct_name: self.ident,
            model_name: self.model,
            fields,
        })
    }
}
