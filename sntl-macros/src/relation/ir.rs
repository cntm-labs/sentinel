use quote::quote;
use syn::{GenericArgument, Ident, ImplItem, PathArguments, ReturnType, Type};

#[derive(Debug)]
pub struct RelationIR {
    pub model_name: Ident,
    pub relations: Vec<SingleRelationIR>,
}

#[derive(Debug)]
pub struct SingleRelationIR {
    pub fn_name: Ident,
    /// UPPERCASE version for the generated constant name.
    pub const_name: Ident,
    pub kind: RelationKindIR,
    pub target_type: Type,
    pub foreign_key: String,
}

#[derive(Debug, Clone, Copy)]
pub enum RelationKindIR {
    HasMany,
    HasOne,
    BelongsTo,
}

impl RelationIR {
    pub fn parse(item_impl: &syn::ItemImpl) -> syn::Result<Self> {
        let model_name = match &*item_impl.self_ty {
            Type::Path(tp) => tp
                .path
                .segments
                .last()
                .ok_or_else(|| syn::Error::new_spanned(&item_impl.self_ty, "expected a type name"))?
                .ident
                .clone(),
            _ => {
                return Err(syn::Error::new_spanned(
                    &item_impl.self_ty,
                    "expected a type name",
                ));
            }
        };

        let mut relations = Vec::new();

        for item in &item_impl.items {
            if let ImplItem::Fn(method) = item {
                let fn_name = &method.sig.ident;

                // Parse return type: HasMany<T>, HasOne<T>, or BelongsTo<T>
                let (kind, target_type) = parse_return_type(&method.sig.output)?;

                // Parse body for foreign_key string literal
                let foreign_key = extract_foreign_key(&method.block)?;

                let const_name = Ident::new(&fn_name.to_string().to_uppercase(), fn_name.span());

                relations.push(SingleRelationIR {
                    fn_name: fn_name.clone(),
                    const_name,
                    kind,
                    target_type,
                    foreign_key,
                });
            }
        }

        Ok(RelationIR {
            model_name,
            relations,
        })
    }
}

fn parse_return_type(ret: &ReturnType) -> syn::Result<(RelationKindIR, Type)> {
    match ret {
        ReturnType::Type(_, ty) => {
            if let Type::Path(tp) = ty.as_ref()
                && let Some(seg) = tp.path.segments.last()
            {
                let kind = match seg.ident.to_string().as_str() {
                    "HasMany" => RelationKindIR::HasMany,
                    "HasOne" => RelationKindIR::HasOne,
                    "BelongsTo" => RelationKindIR::BelongsTo,
                    other => {
                        return Err(syn::Error::new_spanned(
                            &seg.ident,
                            format!("expected HasMany, HasOne, or BelongsTo, got `{other}`"),
                        ));
                    }
                };
                if let PathArguments::AngleBracketed(args) = &seg.arguments
                    && let Some(GenericArgument::Type(target)) = args.args.first()
                {
                    return Ok((kind, target.clone()));
                }
            }
            Err(syn::Error::new_spanned(
                ty,
                "expected HasMany<T>, HasOne<T>, or BelongsTo<T>",
            ))
        }
        _ => Err(syn::Error::new_spanned(
            ret,
            "relation function must have a return type",
        )),
    }
}

fn extract_foreign_key(block: &syn::Block) -> syn::Result<String> {
    // Look for the first string literal in the function body —
    // this is the foreign_key argument to HasMany::new("fk"), etc.
    for stmt in &block.stmts {
        let s = quote!(#stmt).to_string();
        if let Some(start) = s.find('"')
            && let Some(end) = s[start + 1..].find('"')
        {
            return Ok(s[start + 1..start + 1 + end].to_string());
        }
    }
    Err(syn::Error::new_spanned(
        block,
        "could not extract foreign key string from relation body",
    ))
}
