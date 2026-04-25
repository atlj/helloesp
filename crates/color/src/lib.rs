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

    let r8 = hex_pair_to_number(chars.next().unwrap(), chars.next().unwrap());
    let g8 = hex_pair_to_number(chars.next().unwrap(), chars.next().unwrap());
    let b8 = hex_pair_to_number(chars.next().unwrap(), chars.next().unwrap());

    let r5 = (r8 as f64 * 31.0 / 255.0).round() as u16;
    let g6 = (g8 as f64 * 63.0 / 255.0).round() as u16;
    let b5 = (b8 as f64 * 31.0 / 255.0).round() as u16;

    let rgb565: u16 = (r5 << 11) | (g6 << 5) | b5;

    format!("color_core::Color({rgb565}_u16)").parse().unwrap()
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
