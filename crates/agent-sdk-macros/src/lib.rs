//! Optional proc macros for typed Agent SDK toolkit helpers.
//! Generated code calls toolkit builders only; runtime behavior stays in
//! `agent-sdk-core`.

use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::{Ident, Span};
use quote::{format_ident, quote};
use syn::{
    Data, DeriveInput, Expr, Fields, FnArg, ItemFn, Lit, Meta, PatType, ReturnType, Type,
    parse_macro_input,
};

/// Derives `agent_sdk_toolkit::ToolArgs` with a deterministic object schema.
#[proc_macro_derive(ToolArgs)]
pub fn derive_tool_args(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let paths = sdk_paths();
    let name = input.ident;
    let Data::Struct(data) = input.data else {
        return syn::Error::new_spanned(name, "ToolArgs can only be derived for structs")
            .to_compile_error()
            .into();
    };
    let Fields::Named(fields) = data.fields else {
        return syn::Error::new_spanned(name, "ToolArgs requires named struct fields")
            .to_compile_error()
            .into();
    };

    let field_names = fields
        .named
        .iter()
        .map(|field| field.ident.as_ref().expect("named field").to_string())
        .collect::<Vec<_>>();
    let field_schema = fields.named.iter().map(|field| {
        let field_name = field.ident.as_ref().expect("named field").to_string();
        let schema_type = schema_type_for(&field.ty);
        let json_path = &paths.json;
        quote! {
            properties.insert(
                #field_name.to_string(),
                #json_path::json!({ "type": #schema_type }),
            );
        }
    });
    let schema_id = format!("schema.{}", to_snake_case(&name.to_string()));
    let toolkit_path = &paths.toolkit;
    let core_path = &paths.core;
    let json_path = &paths.json;

    quote! {
        impl #toolkit_path::ToolArgs for #name {
            const SCHEMA_ID: &'static str = #schema_id;
            const SCHEMA_VERSION: #core_path::SchemaVersion =
                #core_path::SchemaVersion::new(1, 0, 0);

            fn schema() -> #json_path::Value {
                let mut properties = #json_path::Map::new();
                #(#field_schema)*
                #json_path::json!({
                    "type": "object",
                    "required": [#(#field_names),*],
                    "properties": properties,
                    "additionalProperties": false
                })
            }
        }
    }
    .into()
}

/// Derives `agent_sdk_toolkit::ToolOutput` with the default redacted summary.
#[proc_macro_derive(ToolOutput)]
pub fn derive_tool_output(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let paths = sdk_paths();
    let name = input.ident;
    let toolkit_path = &paths.toolkit;
    quote! {
        impl #toolkit_path::ToolOutput for #name {}
    }
    .into()
}

/// Generates a typed tool builder function for a single-argument sync function.
///
/// `#[agent_tool(name = "lookup_docs", version = "v1")]` on
/// `fn lookup(args: Args) -> ToolResult<Out>` emits `lookup_tool()`.
#[proc_macro_attribute]
pub fn agent_tool(args: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args with syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated);
    let input = parse_macro_input!(item as ItemFn);
    match expand_agent_tool(args.into_iter().collect(), input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

fn expand_agent_tool(args: Vec<Meta>, input: ItemFn) -> syn::Result<proc_macro2::TokenStream> {
    let paths = sdk_paths();
    let mut tool_name = None;
    let mut version = None;
    for meta in args {
        match meta {
            Meta::NameValue(value) if value.path.is_ident("name") => {
                tool_name = string_lit(value.value, "name")?;
            }
            Meta::NameValue(value) if value.path.is_ident("version") => {
                version = string_lit(value.value, "version")?;
            }
            other => {
                return Err(syn::Error::new_spanned(
                    other,
                    "expected name = \"...\" or version = \"...\"",
                ));
            }
        }
    }
    let tool_name = tool_name.ok_or_else(|| {
        syn::Error::new_spanned(&input.sig.ident, "agent_tool requires name = \"...\"")
    })?;
    let version = version.ok_or_else(|| {
        syn::Error::new_spanned(&input.sig.ident, "agent_tool requires version = \"...\"")
    })?;
    let function_name = input.sig.ident.clone();
    let helper_name = format_ident!("{}_tool", function_name);
    let args_ty = first_argument_type(&input)?;
    let output_ty = output_type(&input)?;
    let toolkit_path = &paths.toolkit;
    let core_path = &paths.core;

    Ok(quote! {
        #input

        pub fn #helper_name() -> Result<
            #toolkit_path::TypedTool<#args_ty, #output_ty>,
            #core_path::AgentError,
        > {
            #toolkit_path::TypedTool::<#args_ty, #output_ty>::builder(
                #toolkit_path::ToolIdentity::new(#tool_name, #version)?,
            )
            .sync_handler(|args: #args_ty, _context| #function_name(args))
            .build()
        }
    })
}

struct SdkPaths {
    toolkit: proc_macro2::TokenStream,
    core: proc_macro2::TokenStream,
    json: proc_macro2::TokenStream,
}

fn sdk_paths() -> SdkPaths {
    if let Some(toolkit) = crate_path("agent-sdk-toolkit") {
        let core = quote!(#toolkit::agent_sdk_core);
        let json = quote!(#toolkit::serde_json);
        return SdkPaths {
            toolkit,
            core,
            json,
        };
    }

    let facade = crate_path("clawdia-sdk").unwrap_or_else(|| quote!(::clawdia_sdk));
    let toolkit = quote!(#facade::tools);
    let core = quote!(#facade::core);
    let json = quote!(#facade::tools::serde_json);
    SdkPaths {
        toolkit,
        core,
        json,
    }
}

fn crate_path(package_name: &str) -> Option<proc_macro2::TokenStream> {
    match crate_name(package_name).ok()? {
        FoundCrate::Itself => Some(quote!(crate)),
        FoundCrate::Name(name) => {
            let ident = Ident::new(&name, Span::call_site());
            Some(quote!(::#ident))
        }
    }
}

fn first_argument_type(input: &ItemFn) -> syn::Result<Type> {
    if input.sig.inputs.len() != 1 {
        return Err(syn::Error::new_spanned(
            &input.sig.inputs,
            "agent_tool functions must take exactly one argument",
        ));
    }
    match input.sig.inputs.first().expect("one input") {
        FnArg::Typed(PatType { ty, .. }) => Ok((**ty).clone()),
        FnArg::Receiver(receiver) => Err(syn::Error::new_spanned(
            receiver,
            "agent_tool functions cannot take self",
        )),
    }
}

fn output_type(input: &ItemFn) -> syn::Result<Type> {
    let ReturnType::Type(_, ty) = &input.sig.output else {
        return Err(syn::Error::new_spanned(
            &input.sig.ident,
            "agent_tool functions must return ToolResult<T>",
        ));
    };
    let Type::Path(path) = ty.as_ref() else {
        return Err(syn::Error::new_spanned(ty, "unsupported return type"));
    };
    let Some(segment) = path.path.segments.last() else {
        return Err(syn::Error::new_spanned(ty, "unsupported return type"));
    };
    if segment.ident != "ToolResult" {
        return Err(syn::Error::new_spanned(
            ty,
            "agent_tool functions must return ToolResult<T>",
        ));
    }
    let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
        return Err(syn::Error::new_spanned(
            ty,
            "agent_tool functions must return ToolResult<T>",
        ));
    };
    let Some(syn::GenericArgument::Type(output)) = args.args.first() else {
        return Err(syn::Error::new_spanned(
            ty,
            "agent_tool functions must return ToolResult<T>",
        ));
    };
    Ok(output.clone())
}

fn string_lit(expr: Expr, field: &str) -> syn::Result<Option<String>> {
    let Expr::Lit(expr) = expr else {
        return Err(syn::Error::new_spanned(
            expr,
            format!("{field} must be a string literal"),
        ));
    };
    let Lit::Str(value) = expr.lit else {
        return Err(syn::Error::new_spanned(
            expr,
            format!("{field} must be a string literal"),
        ));
    };
    Ok(Some(value.value()))
}

fn schema_type_for(ty: &Type) -> &'static str {
    match ty {
        Type::Path(path) => {
            let ident = path
                .path
                .segments
                .last()
                .map(|segment| segment.ident.to_string())
                .unwrap_or_default();
            match ident.as_str() {
                "String" | "str" => "string",
                "bool" => "boolean",
                "u8" | "u16" | "u32" | "u64" | "usize" | "i8" | "i16" | "i32" | "i64" | "isize"
                | "f32" | "f64" => "number",
                _ => "object",
            }
        }
        _ => "object",
    }
}

fn to_snake_case(value: &str) -> String {
    let mut out = String::new();
    for (index, character) in value.chars().enumerate() {
        if character.is_uppercase() {
            if index > 0 {
                out.push('_');
            }
            out.extend(character.to_lowercase());
        } else {
            out.push(character);
        }
    }
    out
}
