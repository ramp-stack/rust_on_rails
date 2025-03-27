use proc_macro2::{TokenStream, TokenTree, Literal};
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{
    parse_macro_input, parse_quote, Data, DeriveInput, Fields, GenericParam, Generics, Index, Field, Ident
};

enum ChildType {
    Child(TokenTree),
}

#[proc_macro_derive(Component, attributes(layout, skip))]
pub fn derive_heap_size(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // Parse the input tokens into a syntax tree.
    let input = parse_macro_input!(input as DeriveInput);

    // Used in the quasi-quotation below as `#name`.
    let name = input.ident;

  //// Add a bound `T: HeapSize` to every type parameter T.
  //let generics = add_trait_bounds(input.generics);
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

  //// Generate an expression to sum up the heap size of each field.
  //let sum = heap_size_sum(&input.data);

    let has_tag = |field: &Field, tag: &str| field.attrs.iter().any(|attr| attr.meta.path().get_ident().map(|i| &i.to_string() == tag).unwrap_or_default());

    let (layout, children): (Option<TokenTree>, Vec<ChildType>) = match input.data {
        Data::Struct(struc) => {
            match struc.fields {
                Fields::Named(named) => {todo!()},
                Fields::Unnamed(unnamed) => {
                    unnamed.unnamed.iter().enumerate().fold((None, vec![]), |(mut layout, mut children), (index, field)| {
                        if has_tag(&field, "layout") {
                            layout.replace(TokenTree::Literal(Literal::usize_unsuffixed(index)))
                                .map(|_| {panic!("Component can only have one field tagged as #[layout]");});
                        } else if !has_tag(&field, "skip") {
                            let name = TokenTree::Literal(Literal::usize_unsuffixed(index));
                            panic!("{:?}", field.ty);
                            children.push(ChildType::Child(name));
                        }
                        (layout, children)
                    })
                },
                Fields::Unit => {panic!("Component requires a Layout and at least one child")}
            }
        },
        Data::Enum(enu) => {
            todo!()
        },
        Data::Union(_) => {panic!("Cannot implement Component for a Union")}
    };

    let layout = layout.unwrap_or_else(|| {panic!("Component requires a Layout denoted with #[layout]");});
    let children = TokenStream::from_iter(children.into_iter().map(|child| match child {
        ChildType::Child(name) => quote!{children.push(&self.#name as &dyn Drawable);}
    }));

    //panic!("{layout:?}, {children:#?}");

    let expanded = quote! {
        // The generated impl.
        impl #impl_generics Component for #name #ty_generics #where_clause {
            fn children_mut(&mut self) -> Vec<&mut dyn Drawable> {vec![&mut self.1, &mut self.2]}
            fn children(&self) -> Vec<&dyn Drawable> {
                let mut children = vec![];

                #children

                children
            }
            fn layout(&self) -> &dyn Layout {&self.#layout}
        }
    };
    //panic!("Expanded: {}", expanded);

    // Hand the output tokens back to the compiler.
    proc_macro::TokenStream::from(expanded)
}

//  // Add a bound `T: HeapSize` to every type parameter T.
//  fn add_trait_bounds(mut generics: Generics) -> Generics {
//      for param in &mut generics.params {
//          if let GenericParam::Type(ref mut type_param) = *param {
//              type_param.bounds.push(parse_quote!(heapsize::HeapSize));
//          }
//      }
//      generics
//  }

//  // Generate an expression to sum up the heap size of each field.
//  fn heap_size_sum(data: &Data) -> TokenStream {
//      match *data {
//          Data::Struct(ref data) => {
//              match data.fields {
//                  Fields::Named(ref fields) => {
//                      // Expands to an expression like
//                      //
//                      //     0 + self.x.heap_size() + self.y.heap_size() + self.z.heap_size()
//                      //
//                      // but using fully qualified function call syntax.
//                      //
//                      // We take some care to use the span of each `syn::Field` as
//                      // the span of the corresponding `heap_size_of_children`
//                      // call. This way if one of the field types does not
//                      // implement `HeapSize` then the compiler's error message
//                      // underlines which field it is. An example is shown in the
//                      // readme of the parent directory.
//                      let recurse = fields.named.iter().map(|f| {
//                          let name = &f.ident;
//                          quote_spanned! {f.span()=>
//                              heapsize::HeapSize::heap_size_of_children(&self.#name)
//                          }
//                      });
//                      quote! {
//                          0 #(+ #recurse)*
//                      }
//                  }
//                  Fields::Unnamed(ref fields) => {
//                      // Expands to an expression like
//                      //
//                      //     0 + self.0.heap_size() + self.1.heap_size() + self.2.heap_size()
//                      let recurse = fields.unnamed.iter().enumerate().map(|(i, f)| {
//                          let index = Index::from(i);
//                          quote_spanned! {f.span()=>
//                              heapsize::HeapSize::heap_size_of_children(&self.#index)
//                          }
//                      });
//                      quote! {
//                          0 #(+ #recurse)*
//                      }
//                  }
//                  Fields::Unit => {
//                      // Unit structs cannot own more than 0 bytes of heap memory.
//                      quote!(0)
//                  }
//              }
//          }
//          Data::Enum(_) | Data::Union(_) => unimplemented!(),
//      }
//  }
