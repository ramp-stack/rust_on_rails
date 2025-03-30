use syn::{parse_macro_input,  Data, DeriveInput, Fields, Field, Path, Type, TypePath};
use proc_macro2::{TokenStream, TokenTree, Literal};
use quote::quote;

enum ChildType {
    Vector(TokenTree),
    Child(TokenTree),
    Opt(TokenTree),
}

#[proc_macro_derive(Component, attributes(skip))]
pub fn derive_heap_size(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let has_tag = |field: &Field, tag: &str| field.attrs.iter().any(|attr| attr.meta.path().get_ident().map(|i| &i.to_string() == tag).unwrap_or_default());

    let (layout, children): (TokenTree, Vec<ChildType>) = match input.data {
        Data::Struct(struc) => {
            match struc.fields {
                Fields::Named(_named) => {todo!()},
                Fields::Unnamed(unnamed) => {
                    let mut iterator = unnamed.unnamed.iter().enumerate();
                    iterator.next().unwrap_or_else(|| {panic!("Component requires the first field of the structure to be the layout");});
                    (
                        TokenTree::Literal(Literal::usize_unsuffixed(0)),
                        iterator.flat_map(|(index, field)| {
                            (!has_tag(&field, "skip")).then(|| {
                                match &field.ty {
                                    Type::Path(TypePath{path: Path{segments, ..}, ..}) if segments.first().filter(|s| s.ident.to_string() == "Option".to_string()).is_some() => {
                                        ChildType::Opt(TokenTree::Literal(Literal::usize_unsuffixed(index)))
                                    },
                                    Type::Path(TypePath{path: Path{segments, ..}, ..}) if segments.first().filter(|s| s.ident.to_string() == "Vec".to_string()).is_some() => {
                                        ChildType::Vector(TokenTree::Literal(Literal::usize_unsuffixed(index)))
                                    },
                                    _ => ChildType::Child(TokenTree::Literal(Literal::usize_unsuffixed(index)))
                                }
                            })
                        }).collect()
                    )
                },
                Fields::Unit => {panic!("Component requires a Layout and at least one child")}
            }
        },
        Data::Enum(_enu) => {
            todo!()
        },
        Data::Union(_) => {panic!("Cannot implement Component for a Union")}
    };
    children.is_empty().then(|| {panic!("Component requires at least one child component in the structure");});

    let children_mut = TokenStream::from_iter(children.iter().map(|child| match child {
        ChildType::Vector(name) => quote!{children.extend(self.#name.iter_mut().map(|c| c as &mut dyn Drawable));},
        ChildType::Child(name) => quote!{children.push(&mut self.#name as &mut dyn Drawable);},
        ChildType::Opt(name) => quote!{if let Some(item) = self.#name.as_mut() {children.push(item as &mut dyn Drawable);}}
    }));
    let children = TokenStream::from_iter(children.iter().map(|child| match child {
        ChildType::Vector(name) => quote!{children.extend(self.#name.iter().map(|c| c as &dyn Drawable));},
        ChildType::Child(name) => quote!{children.push(&self.#name as &dyn Drawable);},
        ChildType::Opt(name) => quote!{if let Some(item) = self.#name.as_ref() {children.push(item as &dyn Drawable);}}
    }));

    let expanded = quote! {
        // The generated impl.
        impl #impl_generics Component for #name #ty_generics #where_clause {
            fn children_mut(&mut self) -> Vec<&mut dyn Drawable> {
                let mut children = vec![];

                #children_mut

                children
            }
            fn children(&self) -> Vec<&dyn Drawable> {
                let mut children = vec![];

                #children

                children
            }
            fn layout(&self) -> &dyn Layout {&self.#layout}
        }
    };
    proc_macro::TokenStream::from(expanded)
}
