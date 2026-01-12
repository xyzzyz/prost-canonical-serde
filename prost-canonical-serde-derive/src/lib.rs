//! Proc macros that derive canonical JSON serde implementations for prost types.
//!
//! These derives implement `serde::Serialize` and `serde::Deserialize` using
//! canonical protobuf JSON rules, so callers can keep using `serde_json`
//! normally.
//!
//! # Example
//! ```rust,ignore
//! use prost_canonical_serde::{CanonicalDeserialize, CanonicalSerialize};
//!
//! #[derive(CanonicalSerialize, CanonicalDeserialize)]
//! struct Example {
//!     #[prost(int32, tag = "1")]
//!     #[prost_canonical_serde(proto_name = "value", json_name = "value")]
//!     value: i32,
//! }
//!
//! let json = serde_json::to_string(&Example { value: 1 }).unwrap();
//! ```
use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, spanned::Spanned, Attribute, Data, DeriveInput, Fields, Ident, LitStr, Path,
    Type, TypePath,
};

/// Derives `CanonicalSerialize` and `serde::Serialize` for prost messages.
#[proc_macro_derive(CanonicalSerialize, attributes(prost, prost_canonical_serde))]
pub fn derive_canonical_serialize(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand_serialize(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Derives `CanonicalDeserialize` and `serde::Deserialize` for prost messages.
#[proc_macro_derive(CanonicalDeserialize, attributes(prost, prost_canonical_serde))]
pub fn derive_canonical_deserialize(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand_deserialize(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn expand_serialize(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    match &input.data {
        Data::Struct(data) => expand_serialize_struct(input, data),
        Data::Enum(data) => expand_serialize_enum(input, data),
        Data::Union(_) => Err(syn::Error::new(
            input.span(),
            "CanonicalSerialize does not support unions",
        )),
    }
}

fn expand_deserialize(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    match &input.data {
        Data::Struct(data) => expand_deserialize_struct(input, data),
        Data::Enum(data) => Ok(expand_deserialize_enum(input, data)),
        Data::Union(_) => Err(syn::Error::new(
            input.span(),
            "CanonicalDeserialize does not support unions",
        )),
    }
}

fn expand_serialize_struct(
    input: &DeriveInput,
    data: &syn::DataStruct,
) -> syn::Result<proc_macro2::TokenStream> {
    let name = &input.ident;
    let fields = extract_fields(&data.fields)?;
    let mut field_serializers = Vec::new();

    for field in &fields {
        field_serializers.push(serialize_field(field));
    }

    Ok(quote! {
        impl ::prost_canonical_serde::CanonicalSerialize for #name {
            fn serialize_canonical<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: ::serde::Serializer,
            {
                use ::serde::ser::SerializeMap;
                let mut map = serializer.serialize_map(None)?;
                #(#field_serializers)*
                map.end()
            }
        }

        impl ::serde::Serialize for #name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: ::serde::Serializer,
            {
                <Self as ::prost_canonical_serde::CanonicalSerialize>::serialize_canonical(
                    self,
                    serializer,
                )
            }
        }
    })
}

fn expand_deserialize_struct(
    input: &DeriveInput,
    data: &syn::DataStruct,
) -> syn::Result<proc_macro2::TokenStream> {
    let name = &input.ident;
    let fields = extract_fields(&data.fields)?;
    let mut field_inits = Vec::new();
    let mut field_names = Vec::new();
    let mut match_arms = Vec::new();
    let mut oneof_checks = Vec::new();

    for field in &fields {
        let ident = field.ident.clone();
        field_names.push(ident.clone());
        field_inits.push(init_field(field));

        if field.is_oneof {
            let oneof_type = field
                .oneof_type
                .as_ref()
                .ok_or_else(|| syn::Error::new(ident.span(), "oneof field must be Option"))?;
            oneof_checks.push(quote! {
                match <#oneof_type as ::prost_canonical_serde::ProstOneof>::try_deserialize(
                    key,
                    &mut map,
                )? {
                    ::prost_canonical_serde::OneofMatch::Matched(Some(value)) => {
                        if #ident.is_some() {
                            return Err(::serde::de::Error::custom("multiple oneof fields set"));
                        }
                        #ident = Some(value);
                        continue;
                    }
                    ::prost_canonical_serde::OneofMatch::Matched(None) => {
                        continue;
                    }
                    ::prost_canonical_serde::OneofMatch::NoMatch => {}
                }
            });
        } else {
            match_arms.push(deserialize_match_arm(field)?);
        }
    }

    Ok(quote! {
        impl ::prost_canonical_serde::CanonicalDeserialize for #name {
            fn deserialize_canonical<'de, D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: ::serde::Deserializer<'de>,
            {
                struct Visitor;

                impl<'de> ::serde::de::Visitor<'de> for Visitor {
                    type Value = #name;

                    fn expecting(&self, formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                        formatter.write_str("map")
                    }

                    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
                    where
                        A: ::serde::de::MapAccess<'de>,
                    {
                        #(#field_inits)*

                        while let Some(key) = map.next_key::<::alloc::borrow::Cow<'de, str>>()? {
                            let key = key.as_ref();
                            #(#oneof_checks)*
                            match key {
                                #(#match_arms)*
                                _ => {
                                    let _ = map.next_value::<::serde::de::IgnoredAny>()?;
                                }
                            }
                        }

                        Ok(#name {
                            #(#field_names),*
                        })
                    }
                }

                deserializer.deserialize_map(Visitor)
            }
        }

        impl<'de> ::serde::Deserialize<'de> for #name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: ::serde::Deserializer<'de>,
            {
                <Self as ::prost_canonical_serde::CanonicalDeserialize>::deserialize_canonical(
                    deserializer,
                )
            }
        }
    })
}

fn expand_serialize_enum(
    input: &DeriveInput,
    data: &syn::DataEnum,
) -> syn::Result<proc_macro2::TokenStream> {
    let name = &input.ident;
    if is_oneof_enum(data) {
        let oneof_impl = expand_oneof_impl(input, data)?;
        return Ok(quote! {
            #oneof_impl
            impl ::prost_canonical_serde::CanonicalSerialize for #name {
                fn serialize_canonical<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                where
                    S: ::serde::Serializer,
                {
                    use ::serde::ser::SerializeMap;
                    let mut map = serializer.serialize_map(None)?;
                    <Self as ::prost_canonical_serde::ProstOneof>::serialize_field(self, &mut map)?;
                    map.end()
                }
            }

            impl ::serde::Serialize for #name {
                fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                where
                    S: ::serde::Serializer,
                {
                    <Self as ::prost_canonical_serde::CanonicalSerialize>::serialize_canonical(
                        self,
                        serializer,
                    )
                }
            }
        });
    }

    Ok(quote! {
        impl ::prost_canonical_serde::ProstEnum for #name {
            fn from_i32(value: i32) -> ::core::option::Option<Self> {
                Self::try_from(value).ok()
            }

            fn from_str_name(value: &str) -> ::core::option::Option<Self> {
                #name::from_str_name(value)
            }

            fn as_str_name(&self) -> &'static str {
                self.as_str_name()
            }

            fn as_i32(&self) -> i32 {
                *self as i32
            }
        }

        impl ::prost_canonical_serde::CanonicalSerialize for #name {
            fn serialize_canonical<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: ::serde::Serializer,
            {
                serializer.serialize_str(self.as_str_name())
            }
        }

        impl ::serde::Serialize for #name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: ::serde::Serializer,
            {
                <Self as ::prost_canonical_serde::CanonicalSerialize>::serialize_canonical(
                    self,
                    serializer,
                )
            }
        }
    })
}

fn expand_deserialize_enum(input: &DeriveInput, data: &syn::DataEnum) -> proc_macro2::TokenStream {
    let name = &input.ident;
    if is_oneof_enum(data) {
        return quote! {
            impl ::prost_canonical_serde::CanonicalDeserialize for #name {
                fn deserialize_canonical<'de, D>(deserializer: D) -> Result<Self, D::Error>
                where
                    D: ::serde::Deserializer<'de>,
                {
                    struct Visitor;

                    impl<'de> ::serde::de::Visitor<'de> for Visitor {
                        type Value = #name;

                        fn expecting(&self, formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                            formatter.write_str("map")
                        }

                        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
                        where
                            A: ::serde::de::MapAccess<'de>,
                        {
                            let mut found = None;
                            while let Some(key) = map.next_key::<::alloc::borrow::Cow<'de, str>>()? {
                                let key = key.as_ref();
                                match <#name as ::prost_canonical_serde::ProstOneof>::try_deserialize(
                                    key,
                                    &mut map,
                                )? {
                                    ::prost_canonical_serde::OneofMatch::Matched(Some(value)) => {
                                        if found.is_some() {
                                            return Err(::serde::de::Error::custom(
                                                "multiple oneof fields set",
                                            ));
                                        }
                                        found = Some(value);
                                        continue;
                                    }
                                    ::prost_canonical_serde::OneofMatch::Matched(None) => {
                                        continue;
                                    }
                                    ::prost_canonical_serde::OneofMatch::NoMatch => {
                                        let _ = map.next_value::<::serde::de::IgnoredAny>()?;
                                    }
                                }
                            }

                            found.ok_or_else(|| ::serde::de::Error::custom("expected oneof field"))
                        }
                    }

                    deserializer.deserialize_map(Visitor)
            }
        }

        impl<'de> ::serde::Deserialize<'de> for #name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: ::serde::Deserializer<'de>,
            {
                <Self as ::prost_canonical_serde::CanonicalDeserialize>::deserialize_canonical(
                    deserializer,
                )
            }
        }
        };
    }

    quote! {
        impl ::prost_canonical_serde::CanonicalDeserialize for #name {
            fn deserialize_canonical<'de, D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: ::serde::Deserializer<'de>,
            {
                let value = <::prost_canonical_serde::CanonicalEnumValue<#name> as ::serde::Deserialize>::deserialize(
                    deserializer,
                )?
                .0;
                #name::from_i32(value)
                    .ok_or_else(|| ::serde::de::Error::custom("unknown enum number"))
            }
        }

        impl<'de> ::serde::Deserialize<'de> for #name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: ::serde::Deserializer<'de>,
            {
                <Self as ::prost_canonical_serde::CanonicalDeserialize>::deserialize_canonical(
                    deserializer,
                )
            }
        }
    }
}

fn expand_oneof_impl(
    input: &DeriveInput,
    data: &syn::DataEnum,
) -> syn::Result<proc_macro2::TokenStream> {
    let name = &input.ident;
    let mut serialize_arms = Vec::new();
    let mut deserialize_arms = Vec::new();

    for variant in &data.variants {
        let ident = &variant.ident;
        let (proto_name_attr, json_name_attr) = parse_canonical_attrs(&variant.attrs)?;
        let (value_ty, kind, enum_path) = parse_variant(variant)?;
        let fallback = lower_camel(&ident.to_string());
        let proto_name = proto_name_attr.unwrap_or_else(|| fallback.clone());
        let json_name = json_name_attr.unwrap_or_else(|| fallback.clone());
        let json_name_literal = LitStr::new(&json_name, ident.span());
        let proto_name_literal = LitStr::new(&proto_name, ident.span());
        let value_ident = Ident::new("value", ident.span());

        let serialize_expr = serialize_value_expr(&kind, &value_ident, enum_path.as_ref());
        let deserialize_expr = if let Kind::Enum(path) = &kind {
            let path = enum_path.as_ref().unwrap_or(path);
            quote! {
                map.next_value::<::prost_canonical_serde::CanonicalEnumOption<#path>>()?.0
            }
        } else {
            quote! {
                map.next_value::<::prost_canonical_serde::CanonicalOption<#value_ty>>()?.0
            }
        };

        serialize_arms.push(quote! {
            Self::#ident(#value_ident) => {
                let value = #serialize_expr;
                map.serialize_entry(#json_name_literal, &value)?;
            }
        });

        let match_pat = if json_name == proto_name {
            quote! { #json_name_literal }
        } else {
            quote! { #json_name_literal | #proto_name_literal }
        };

        deserialize_arms.push(quote! {
            #match_pat => {
                let value = #deserialize_expr;
                Ok(::prost_canonical_serde::OneofMatch::Matched(value.map(Self::#ident)))
            }
        });
    }

    Ok(quote! {
        impl ::prost_canonical_serde::ProstOneof for #name {
            fn serialize_field<S>(&self, map: &mut S) -> Result<(), S::Error>
            where
                S: ::serde::ser::SerializeMap,
            {
                match self {
                    #(#serialize_arms),*
                }
                Ok(())
            }

            fn try_deserialize<'de, A>(key: &str, map: &mut A) -> Result<::prost_canonical_serde::OneofMatch<Self>, A::Error>
            where
                A: ::serde::de::MapAccess<'de>,
            {
                match key {
                    #(#deserialize_arms),*,
                    _ => Ok(::prost_canonical_serde::OneofMatch::NoMatch),
                }
            }
        }
    })
}

fn serialize_field(field: &FieldInfo) -> proc_macro2::TokenStream {
    let ident = &field.ident;
    let json_name = LitStr::new(&field.json_name, ident.span());

    if field.is_oneof {
        return quote! {
            if let Some(value) = &self.#ident {
                ::prost_canonical_serde::ProstOneof::serialize_field(value, &mut map)?;
            }
        };
    }

    match &field.kind {
        Kind::Option(inner) => {
            let value_expr = serialize_value_expr(
                inner,
                &Ident::new("value", ident.span()),
                field.enum_path.as_ref(),
            );
            quote! {
                if let Some(value) = &self.#ident {
                    let value = #value_expr;
                    map.serialize_entry(#json_name, &value)?;
                }
            }
        }
        Kind::Vec(inner) => {
            let value_stmt = if let Kind::Enum(path) = inner.as_ref() {
                quote! {
                    let value = ::prost_canonical_serde::CanonicalEnumSeq::<#path>::new(&self.#ident);
                    map.serialize_entry(#json_name, &value)?;
                }
            } else {
                quote! {
                    let value = ::prost_canonical_serde::CanonicalSeq::new(&self.#ident);
                    map.serialize_entry(#json_name, &value)?;
                }
            };

            quote! {
                if !self.#ident.is_empty() {
                    #value_stmt
                }
            }
        }
        Kind::Map(_, _, value_kind) => {
            let value_stmt = if let Kind::Enum(path) = value_kind.as_ref() {
                quote! {
                    let value = ::prost_canonical_serde::CanonicalEnumMapRef::<#path, _>::new(&self.#ident);
                    map.serialize_entry(#json_name, &value)?;
                }
            } else {
                quote! {
                    let value = ::prost_canonical_serde::CanonicalMapRef::new(&self.#ident);
                    map.serialize_entry(#json_name, &value)?;
                }
            };

            quote! {
                if !self.#ident.is_empty() {
                    #value_stmt
                }
            }
        }
        _ => {
            let value_expr = serialize_value_expr(
                &field.kind,
                &Ident::new("value", ident.span()),
                field.enum_path.as_ref(),
            );
            let field_expr = quote! { self.#ident };
            let default_check = default_check_expr(&field.kind, &field_expr);
            quote! {
                if #default_check {
                    let value = &self.#ident;
                    let value = #value_expr;
                    map.serialize_entry(#json_name, &value)?;
                }
            }
        }
    }
}

fn init_field(field: &FieldInfo) -> proc_macro2::TokenStream {
    let ident = &field.ident;

    if field.is_oneof {
        return quote! {
            let mut #ident = ::core::option::Option::None;
        };
    }

    match &field.kind {
        Kind::Option(_) => quote! {
            let mut #ident = ::core::option::Option::None;
        },
        Kind::Vec(_) => quote! {
            let mut #ident = ::alloc::vec::Vec::new();
        },
        Kind::Map(map_kind, _, _) => {
            let map_new = map_new_expr(map_kind);
            quote! {
                let mut #ident = #map_new;
            }
        }
        _ => {
            let default_expr = default_value_expr(&field.kind);
            quote! {
                let mut #ident = #default_expr;
            }
        }
    }
}

fn deserialize_match_arm(field: &FieldInfo) -> syn::Result<proc_macro2::TokenStream> {
    let ident = &field.ident;
    let json_name = LitStr::new(&field.json_name, ident.span());
    let proto_name = LitStr::new(&field.proto_name, ident.span());
    let ty = &field.ty;
    let match_pat = if field.json_name == field.proto_name {
        quote! { #json_name }
    } else {
        quote! { #json_name | #proto_name }
    };

    match &field.kind {
        Kind::Option(inner) => {
            let inner_ty = field
                .option_inner
                .as_ref()
                .ok_or_else(|| syn::Error::new(ident.span(), "missing Option inner type"))?;
            if is_prost_value_type(inner_ty) {
                return Ok(quote! {
                    #match_pat => {
                        #ident = Some(
                            map.next_value::<::prost_canonical_serde::CanonicalValue<#inner_ty>>()?
                                .0,
                        );
                    }
                });
            }
            let value_expr = if let Kind::Enum(path) = inner.as_ref() {
                let path = field.enum_path.as_ref().unwrap_or(path);
                quote! {
                    map.next_value::<::prost_canonical_serde::CanonicalEnumOption<#path>>()?.0
                }
            } else {
                quote! {
                    map.next_value::<::prost_canonical_serde::CanonicalOption<#inner_ty>>()?.0
                }
            };
            Ok(quote! {
                #match_pat => {
                    #ident = #value_expr;
                }
            })
        }
        Kind::Vec(inner) => {
            if let Kind::Enum(path) = inner.as_ref() {
                return Ok(quote! {
                    #match_pat => {
                        #ident = map
                            .next_value::<::prost_canonical_serde::CanonicalEnumVec<#path>>()?
                            .0;
                    }
                });
            }
            let inner_ty = field
                .vec_inner
                .as_ref()
                .ok_or_else(|| syn::Error::new(ident.span(), "missing Vec inner type"))?;
            Ok(quote! {
                #match_pat => {
                    #ident = map
                        .next_value::<::prost_canonical_serde::CanonicalVec<#inner_ty>>()?
                        .0;
                }
            })
        }
        Kind::Map(_, _, value_kind) => {
            let value_expr = if let Kind::Enum(path) = value_kind.as_ref() {
                quote! {
                    map.next_value::<::prost_canonical_serde::CanonicalEnumMap<#path, #ty>>()?.0
                }
            } else {
                quote! {
                    map.next_value::<::prost_canonical_serde::CanonicalMap<#ty>>()?.0
                }
            };
            Ok(quote! {
                #match_pat => {
                    #ident = #value_expr;
                }
            })
        }
        Kind::Enum(path) => {
            let path = field.enum_path.as_ref().unwrap_or(path);
            Ok(quote! {
                #match_pat => {
                    if let Some(value) = map
                        .next_value::<::prost_canonical_serde::CanonicalEnumOption<#path>>()?
                        .0
                    {
                        #ident = value;
                    }
                }
            })
        }
        _ => Ok(quote! {
            #match_pat => {
                if let Some(value) = map
                    .next_value::<::prost_canonical_serde::CanonicalOption<#ty>>()?
                    .0
                {
                    #ident = value;
                }
            }
        }),
    }
}

fn serialize_value_expr(
    kind: &Kind,
    ident: &Ident,
    enum_path: Option<&Path>,
) -> proc_macro2::TokenStream {
    if let Kind::Enum(path) = kind {
        let path = enum_path.unwrap_or(path);
        quote! {
            ::prost_canonical_serde::CanonicalEnum::<#path>::new(*#ident)
        }
    } else {
        quote! { ::prost_canonical_serde::Canonical::new(#ident) }
    }
}

fn map_new_expr(kind: &MapKind) -> proc_macro2::TokenStream {
    match kind {
        MapKind::Hash => quote! { ::std::collections::HashMap::new() },
        MapKind::BTree => quote! { ::alloc::collections::BTreeMap::new() },
    }
}

fn default_value_expr(kind: &Kind) -> proc_macro2::TokenStream {
    match kind {
        Kind::Scalar(ScalarKind::Bool) => quote! { false },
        Kind::Scalar(ScalarKind::I32 | ScalarKind::U32 | ScalarKind::I64 | ScalarKind::U64)
        | Kind::Enum(_) => quote! { 0 },
        Kind::Scalar(ScalarKind::F32 | ScalarKind::F64) => quote! { 0.0 },
        Kind::Scalar(ScalarKind::String) => quote! { ::alloc::string::String::new() },
        Kind::Bytes | Kind::Vec(_) => quote! { ::alloc::vec::Vec::new() },
        Kind::Map(map_kind, _, _) => map_new_expr(map_kind),
        Kind::Timestamp => quote! { ::prost_types::Timestamp::default() },
        Kind::Duration => quote! { ::prost_types::Duration::default() },
        Kind::Message => quote! { ::core::default::Default::default() },
        Kind::Option(_) => quote! { None },
    }
}

fn default_check_expr(kind: &Kind, field: &proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    match kind {
        Kind::Scalar(ScalarKind::Bool) => quote! { #field },
        Kind::Scalar(ScalarKind::I32 | ScalarKind::U32 | ScalarKind::I64 | ScalarKind::U64)
        | Kind::Enum(_) => quote! { #field != 0 },
        Kind::Scalar(ScalarKind::F32 | ScalarKind::F64) => quote! { #field != 0.0 },
        Kind::Scalar(ScalarKind::String) | Kind::Bytes | Kind::Vec(_) | Kind::Map(_, _, _) => {
            quote! { !#field.is_empty() }
        }
        Kind::Timestamp | Kind::Duration | Kind::Message => quote! { true },
        Kind::Option(_) => quote! { #field.is_some() },
    }
}

fn is_prost_value_type(ty: &Type) -> bool {
    let Type::Path(path) = ty else { return false };
    let last = path.path.segments.last().map(|seg| seg.ident.to_string());
    if last.as_deref() != Some("Value") {
        return false;
    }
    path.path
        .segments
        .iter()
        .any(|seg| seg.ident == "prost_types")
}

fn extract_fields(fields: &Fields) -> syn::Result<Vec<FieldInfo>> {
    match fields {
        Fields::Named(named) => named.named.iter().map(FieldInfo::from_field).collect(),
        Fields::Unnamed(_) | Fields::Unit => Err(syn::Error::new(
            fields.span(),
            "CanonicalSerialize requires named fields",
        )),
    }
}

fn parse_variant(variant: &syn::Variant) -> syn::Result<(Type, Kind, Option<Path>)> {
    let fields = match &variant.fields {
        Fields::Unnamed(fields) if fields.unnamed.len() == 1 => &fields.unnamed[0],
        _ => {
            return Err(syn::Error::new(
                variant.span(),
                "oneof variants must be tuple variants with one field",
            ))
        }
    };

    let (is_oneof, enum_path) = parse_prost_attrs(&variant.attrs)?;
    if is_oneof {
        return Err(syn::Error::new(
            variant.span(),
            "unexpected oneof attribute on variant",
        ));
    }

    let mut kind = classify_type(&fields.ty)?;
    if let Some(enum_path) = enum_path.clone() {
        kind = apply_enum(kind, enum_path);
    }

    Ok((fields.ty.clone(), kind, enum_path))
}

fn parse_prost_attrs(attrs: &[Attribute]) -> syn::Result<(bool, Option<Path>)> {
    let mut is_oneof = false;
    let mut enum_path = None;

    for attr in attrs {
        if !attr.path().is_ident("prost") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("oneof") {
                if meta.input.peek(syn::Token![=]) {
                    let value = meta.value()?;
                    let _ = value.parse::<syn::Lit>()?;
                }
                is_oneof = true;
                return Ok(());
            }
            if meta.path.is_ident("enumeration") {
                let value = meta.value()?;
                let lit: LitStr = value.parse()?;
                let path = syn::parse_str::<Path>(&lit.value())?;
                enum_path = Some(path);
                return Ok(());
            }
            if meta.path.is_ident("btree_map")
                || meta.path.is_ident("map")
                || meta.path.is_ident("hash_map")
            {
                let value = meta.value()?;
                let lit: LitStr = value.parse()?;
                if let Some(path) = parse_enum_path_from_map(&lit.value())? {
                    enum_path = Some(path);
                }
                return Ok(());
            }
            if meta.input.peek(syn::Token![=]) {
                let value = meta.value()?;
                let _ = value.parse::<syn::Lit>()?;
            }
            Ok(())
        })?;
    }

    Ok((is_oneof, enum_path))
}

fn parse_enum_path_from_map(value: &str) -> syn::Result<Option<Path>> {
    let needle = "enumeration(";
    let start = match value.find(needle) {
        Some(index) => index + needle.len(),
        None => return Ok(None),
    };
    let end = value[start..]
        .find(')')
        .ok_or_else(|| syn::Error::new(proc_macro2::Span::call_site(), "invalid map enum"))?;
    let path_str = value[start..start + end].trim();
    if path_str.is_empty() {
        return Ok(None);
    }
    let path = syn::parse_str::<Path>(path_str)?;
    Ok(Some(path))
}

fn is_oneof_enum(data: &syn::DataEnum) -> bool {
    data.variants.iter().any(|variant| {
        variant
            .attrs
            .iter()
            .any(|attr| attr.path().is_ident("prost"))
    })
}

fn classify_type(ty: &Type) -> syn::Result<Kind> {
    if let Some(inner) = extract_generic(ty, "Option", 0) {
        return Ok(Kind::Option(Box::new(classify_type(inner)?)));
    }

    if let Some(inner) = extract_generic(ty, "Vec", 0) {
        if is_u8(inner) {
            return Ok(Kind::Bytes);
        }
        return Ok(Kind::Vec(Box::new(classify_type(inner)?)));
    }

    if let Some((map_kind, key, value)) = extract_map_types(ty) {
        let key_kind = classify_key(key)?;
        let value_kind = classify_type(value)?;
        return Ok(Kind::Map(map_kind, key_kind, Box::new(value_kind)));
    }

    if is_bool(ty) {
        return Ok(Kind::Scalar(ScalarKind::Bool));
    }
    if is_i32(ty) {
        return Ok(Kind::Scalar(ScalarKind::I32));
    }
    if is_u32(ty) {
        return Ok(Kind::Scalar(ScalarKind::U32));
    }
    if is_i64(ty) {
        return Ok(Kind::Scalar(ScalarKind::I64));
    }
    if is_u64(ty) {
        return Ok(Kind::Scalar(ScalarKind::U64));
    }
    if is_f32(ty) {
        return Ok(Kind::Scalar(ScalarKind::F32));
    }
    if is_f64(ty) {
        return Ok(Kind::Scalar(ScalarKind::F64));
    }
    if is_string(ty) {
        return Ok(Kind::Scalar(ScalarKind::String));
    }
    if is_timestamp(ty) {
        return Ok(Kind::Timestamp);
    }
    if is_duration(ty) {
        return Ok(Kind::Duration);
    }

    Ok(Kind::Message)
}

fn classify_key(ty: &Type) -> syn::Result<KeyKind> {
    if is_string(ty) {
        return Ok(KeyKind::String);
    }
    if is_bool(ty) {
        return Ok(KeyKind::Bool);
    }
    if is_i32(ty) {
        return Ok(KeyKind::I32);
    }
    if is_i64(ty) {
        return Ok(KeyKind::I64);
    }
    if is_u32(ty) {
        return Ok(KeyKind::U32);
    }
    if is_u64(ty) {
        return Ok(KeyKind::U64);
    }

    Err(syn::Error::new(ty.span(), "unsupported map key type"))
}

fn apply_enum(kind: Kind, enum_path: Path) -> Kind {
    match kind {
        Kind::Scalar(ScalarKind::I32) => Kind::Enum(enum_path),
        Kind::Vec(inner) => match *inner {
            Kind::Scalar(ScalarKind::I32) => Kind::Vec(Box::new(Kind::Enum(enum_path))),
            other => Kind::Vec(Box::new(other)),
        },
        Kind::Option(inner) => match *inner {
            Kind::Scalar(ScalarKind::I32) => Kind::Option(Box::new(Kind::Enum(enum_path))),
            other => Kind::Option(Box::new(other)),
        },
        Kind::Map(map_kind, key_kind, value_kind) => match *value_kind {
            Kind::Scalar(ScalarKind::I32) => {
                Kind::Map(map_kind, key_kind, Box::new(Kind::Enum(enum_path)))
            }
            other => Kind::Map(map_kind, key_kind, Box::new(other)),
        },
        other => other,
    }
}

fn extract_generic<'a>(ty: &'a Type, name: &str, index: usize) -> Option<&'a Type> {
    let Type::Path(TypePath { path, .. }) = ty else {
        return None;
    };
    let segment = path.segments.last()?;
    if segment.ident != name {
        return None;
    }
    let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
        return None;
    };
    let arg = args.args.iter().nth(index)?;
    if let syn::GenericArgument::Type(ty) = arg {
        Some(ty)
    } else {
        None
    }
}

fn extract_map_types(ty: &Type) -> Option<(MapKind, &Type, &Type)> {
    let Type::Path(TypePath { path, .. }) = ty else {
        return None;
    };
    let segment = path.segments.last()?;
    let map_kind = if segment.ident == "HashMap" {
        MapKind::Hash
    } else if segment.ident == "BTreeMap" {
        MapKind::BTree
    } else {
        return None;
    };
    let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
        return None;
    };
    let mut iter = args.args.iter();
    let key = iter.next()?;
    let value = iter.next()?;
    match (key, value) {
        (syn::GenericArgument::Type(key), syn::GenericArgument::Type(value)) => {
            Some((map_kind, key, value))
        }
        _ => None,
    }
}

fn is_bool(ty: &Type) -> bool {
    path_ends_with_ident(ty, "bool")
}

fn is_i32(ty: &Type) -> bool {
    path_ends_with_ident(ty, "i32")
}

fn is_u32(ty: &Type) -> bool {
    path_ends_with_ident(ty, "u32")
}

fn is_i64(ty: &Type) -> bool {
    path_ends_with_ident(ty, "i64")
}

fn is_u64(ty: &Type) -> bool {
    path_ends_with_ident(ty, "u64")
}

fn is_f32(ty: &Type) -> bool {
    path_ends_with_ident(ty, "f32")
}

fn is_f64(ty: &Type) -> bool {
    path_ends_with_ident(ty, "f64")
}

fn is_u8(ty: &Type) -> bool {
    path_ends_with_ident(ty, "u8")
}

fn is_string(ty: &Type) -> bool {
    path_ends_with_ident(ty, "String")
}

fn is_timestamp(ty: &Type) -> bool {
    path_ends_with(ty, &["prost_types", "Timestamp"])
}

fn is_duration(ty: &Type) -> bool {
    path_ends_with(ty, &["prost_types", "Duration"])
}

fn path_ends_with_ident(ty: &Type, ident: &str) -> bool {
    let Type::Path(TypePath { path, .. }) = ty else {
        return false;
    };
    path.segments.last().is_some_and(|seg| seg.ident == ident)
}

fn path_ends_with(ty: &Type, idents: &[&str]) -> bool {
    let Type::Path(TypePath { path, .. }) = ty else {
        return false;
    };
    if path.segments.len() < idents.len() {
        return false;
    }
    let start = path.segments.len() - idents.len();
    path.segments
        .iter()
        .skip(start)
        .zip(idents)
        .all(|(seg, ident)| seg.ident == ident)
}

fn lower_camel(name: &str) -> String {
    let mut result = String::new();
    let mut iter = name.split('_');
    if let Some(first) = iter.next() {
        let mut chars = first.chars();
        if let Some(first_char) = chars.next() {
            result.push(first_char.to_ascii_lowercase());
            result.push_str(chars.as_str());
        }
    }
    for part in iter {
        if part.is_empty() {
            continue;
        }
        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            result.push(first.to_ascii_uppercase());
            result.push_str(chars.as_str());
        }
    }
    result
}

fn to_json_name(name: &str) -> String {
    let mut result = String::with_capacity(name.len());
    let mut capitalize_next = false;

    for ch in name.chars() {
        if ch == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(ch.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            result.push(ch);
        }
    }

    result
}

#[derive(Clone)]
struct FieldInfo {
    ident: Ident,
    ty: Type,
    kind: Kind,
    enum_path: Option<Path>,
    is_oneof: bool,
    json_name: String,
    proto_name: String,
    oneof_type: Option<Type>,
    option_inner: Option<Type>,
    vec_inner: Option<Type>,
}

impl FieldInfo {
    fn from_field(field: &syn::Field) -> syn::Result<Self> {
        let ident = field
            .ident
            .clone()
            .ok_or_else(|| syn::Error::new(field.span(), "expected named field"))?;
        let (is_oneof, enum_path) = parse_prost_attrs(&field.attrs)?;
        let (proto_name_attr, json_name_attr) = parse_canonical_attrs(&field.attrs)?;
        let mut kind = classify_type(&field.ty)?;
        let mut oneof_type = None;
        let option_inner = extract_generic(&field.ty, "Option", 0).cloned();
        let vec_inner = extract_generic(&field.ty, "Vec", 0).cloned();

        if let Some(enum_path) = enum_path.clone() {
            kind = apply_enum(kind, enum_path);
        }

        if is_oneof {
            if let Some(inner) = extract_generic(&field.ty, "Option", 0) {
                oneof_type = Some(inner.clone());
                kind = Kind::Option(Box::new(Kind::Message));
            }
        }

        let proto_name = proto_name_attr.unwrap_or_else(|| ident.to_string());
        let json_name = json_name_attr.unwrap_or_else(|| to_json_name(&proto_name));

        Ok(Self {
            ident,
            ty: field.ty.clone(),
            kind,
            enum_path,
            is_oneof,
            json_name,
            proto_name,
            oneof_type,
            option_inner,
            vec_inner,
        })
    }
}

fn parse_canonical_attrs(attrs: &[Attribute]) -> syn::Result<(Option<String>, Option<String>)> {
    let mut proto_name = None;
    let mut json_name = None;

    for attr in attrs {
        if !attr.path().is_ident("prost_canonical_serde") {
            continue;
        }

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("proto_name") {
                let value: LitStr = meta.value()?.parse()?;
                proto_name = Some(value.value());
            } else if meta.path.is_ident("json_name") {
                let value: LitStr = meta.value()?.parse()?;
                json_name = Some(value.value());
            }
            Ok(())
        })?;
    }

    Ok((proto_name, json_name))
}

#[derive(Clone)]
enum Kind {
    Scalar(ScalarKind),
    Bytes,
    Vec(Box<Kind>),
    Map(MapKind, KeyKind, Box<Kind>),
    Option(Box<Kind>),
    Enum(Path),
    Timestamp,
    Duration,
    Message,
}

#[derive(Clone)]
enum ScalarKind {
    Bool,
    I32,
    U32,
    I64,
    U64,
    F32,
    F64,
    String,
}

#[derive(Clone)]
enum KeyKind {
    String,
    Bool,
    I32,
    I64,
    U32,
    U64,
}

#[derive(Clone)]
enum MapKind {
    Hash,
    BTree,
}
