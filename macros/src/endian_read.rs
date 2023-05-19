use super::macro_args::MacroArgs;
use darling::FromAttributes;
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::{quote, ToTokens};
use syn::{
    parse_macro_input, punctuated::Punctuated, token::Comma, Data, DataStruct, DeriveInput, Field,
    Fields, Type, TypeArray,
};

fn create_field(
    field: &Field,
    field_method: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let field_ident = field.ident.as_ref().expect("Field should have identity");
    let pad_before = match MacroArgs::from_attributes(&field.attrs).ok() {
        Some(MacroArgs { pad_before }) => {
            quote! { ::no_std_io::Cursor::increment_by(&mut stream, #pad_before); }
        }
        _ => quote! {},
    };

    let field_method = match &field.ty {
        Type::Array(TypeArray { elem, .. }) if &elem.to_token_stream().to_string() != "u8" => {
            syn::Ident::new(
                &field_method.to_string().replace("read", "read_array"),
                Span::call_site(),
            )
        }
        _ => syn::Ident::new(&field_method.to_string(), Span::call_site()),
    };

    quote! {
        #pad_before
        let #field_ident = ::no_std_io::StreamReader::#field_method(&mut stream)?;
    }
}

fn create_method_impl(
    fields: &Punctuated<Field, Comma>,
    impl_method: proc_macro2::TokenStream,
    field_method: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let field_tokens = fields
        .iter()
        .map(|field| create_field(field, &field_method))
        .collect::<Vec<proc_macro2::TokenStream>>();
    let field_idents = fields
        .iter()
        .map(|field| field.ident.as_ref().expect("Field should have identity"))
        .collect::<Vec<&Ident>>();

    quote! {
        #[inline(always)]
        fn #impl_method(bytes: &[u8]) -> Result<::no_std_io::ReadOutput<Self>, ::no_std_io::Error> {
            let mut stream = ::no_std_io::StreamContainer::new(bytes);
            #(#field_tokens)*
            let result = Self {
                #(#field_idents),*
            };
            let bytes_read = ::no_std_io::Cursor::get_index(&stream);

            Ok(::no_std_io::ReadOutput::new(result, bytes_read))
        }
    }
}

pub fn impl_endian_read(tokens: TokenStream) -> TokenStream {
    let input = parse_macro_input!(tokens as DeriveInput);

    let named_fields = match input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => fields.named,
        _ => panic!("Only structs can derive EndianRead"),
    };

    let try_read_le = create_method_impl(
        &named_fields,
        quote! { try_read_le },
        quote! { read_stream_le },
    );

    let try_read_be = create_method_impl(
        &named_fields,
        quote! { try_read_be },
        quote! { read_stream_be },
    );

    let name = input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let modified = quote! {
        impl #impl_generics ::no_std_io::EndianRead for #name #ty_generics #where_clause {
            #try_read_le
            #try_read_be
        }
    };

    modified.into()
}
