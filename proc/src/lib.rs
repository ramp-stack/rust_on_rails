use proc_macro::TokenStream;
use proc_macro::TokenTree;
use proc_macro::Delimiter;

#[proc_macro_derive(Component, attributes(skip))]
pub fn derive_component(item: TokenStream) -> TokenStream {
    let items = item.into_iter().find_map(|i| {
        match i {
            TokenTree::Group(group) => {
                match group.delimiter() {
                    Delimiter::Brace => Some(format!("{:#?}", group.stream())),
                    Delimiter::Parenthesis => Some(format!("{:?}", group.stream())),
                    _ => None
                }
            },
            _ => None
        }
    }).expect("Component cannot be derived for struct with no fields");
    let items = items
        .replace("\"", "\\\"")
        .replace("{", "{{").replace("}", "}}");
    format!("
        impl Test {{
            fn answer(&self) {{ println!(\"{items}\"); }}
        }}
    ").parse().unwrap()
}
