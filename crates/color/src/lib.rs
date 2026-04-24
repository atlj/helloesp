use proc_macro::TokenStream;

#[proc_macro]
pub fn color(input: TokenStream) -> TokenStream {
    let mut iter = input.into_iter();
    match iter.next() {
        Some(token) => match token {
            proc_macro::TokenTree::Punct(punct) if punct.as_char() == '#' => {}
            _ => panic!("color needs to start with #"),
        },
        None => panic!("needs a color literal"),
    };

    let hex_str = match iter.next() {
        Some(token) => match token {
            proc_macro::TokenTree::Ident(ident) => ident.to_string(),
            proc_macro::TokenTree::Literal(literal) => literal.to_string(),
            _ => panic!("needs a color literal"),
        },
        None => panic!("needs a color literal"),
    };

    if hex_str.len() != 6 {
        panic!("hex code needs 6 characters")
    }

    let mut chars = hex_str.chars();

    let b = hex_pair_to_number(chars.next().unwrap(), chars.next().unwrap());
    let g = hex_pair_to_number(chars.next().unwrap(), chars.next().unwrap());
    let r = hex_pair_to_number(chars.next().unwrap(), chars.next().unwrap());

    let result = format!("color_core::Color {{ r: {}, g: {}, b: {} }}", r, g, b);
    result.parse().unwrap()
}

fn hex_pair_to_number(high: char, low: char) -> u8 {
    let high_num = hex_char_to_number(high);
    let low_num = hex_char_to_number(low);

    (high_num << 4) + low_num
}

fn hex_char_to_number(char: char) -> u8 {
    match char.to_ascii_uppercase() {
        '0' => 0,
        '1' => 1,
        '2' => 2,
        '3' => 3,
        '4' => 4,
        '5' => 5,
        '6' => 6,
        '7' => 7,
        '8' => 8,
        '9' => 9,
        'A' => 10,
        'B' => 11,
        'C' => 12,
        'D' => 13,
        'E' => 14,
        'F' => 15,
        _ => panic!("Unknown hex char"),
    }
}
