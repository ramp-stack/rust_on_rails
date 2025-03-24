use proc_macro::{TokenStream, TokenTree, Delimiter, Group};


fn split_punct(stream: TokenStream, split: char) -> Vec<TokenStream> {
    let mut result: Vec<TokenStream> = vec![];
    let mut buffer: Vec<TokenTree> = vec![];
    stream.into_iter().for_each(|i| match i {
        TokenTree::Punct(token) if token == split => {
            if !buffer.is_empty() {result.push(TokenStream::from_iter(buffer.drain(..)));}
        },
        token => {buffer.push(token);}
    });

    result
}

fn remove_tags(stream: TokenStream) -> (Vec<Group>, TokenStream) {
    let mut result: Vec<Group> = vec![];
    let mut result_stream = TokenStream::new();
    let mut hashtag = None;
    stream.into_iter().for_each(|i| match i {
        TokenTree::Punct(token) if token == '#' => {hashtag = Some(token);}
        TokenTree::Group(group) if hashtag.is_some() => {hashtag = None; result.push(group);}
        token => {
            result_stream.extend([hashtag.take().map(TokenTree::Punct), Some(token)].into_iter().flatten());
        }
    });

    (result, result_stream)
}


#[proc_macro_derive(Component, attributes(layout, skip))]
pub fn derive_component(item: TokenStream) -> TokenStream {
    let mut layout = None;
    let items = item.into_iter().find_map(|i| {
        match i {
            TokenTree::Group(group) => {
                match group.delimiter() {
                    Delimiter::Brace => Some(format!("{:#?}", group.stream())),
                    Delimiter::Parenthesis => {
                        let items = split_punct(group.stream(), ',')
                            .into_iter().enumerate().flat_map(|(index, line)| {
                            let (tags, stream) = remove_tags(line);
                            if tags.iter().any(|tag| tag.to_string() == "[layout]") {
                                if layout.is_some() {panic!("A Component can only have one layout")}
                                layout = Some(stream.into_iter().next().unwrap());
                                //Some(format!("Layout: {stream}"))
                                Some("found".to_string())
                            } else if tags.iter().any(|tag| tag.to_string() == "[skip]") {
                                None
                            } else {
                                Some(format!("Component: {index}"))
                            }
                        }).collect::<Vec<_>>();
                        Some(format!("{:#?}", items))
                    },
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
