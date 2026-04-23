//! rule — regex patterns, extractors and validators ported from
//! `jionlp/rule/`.

pub mod checker;
pub mod cleaners;
pub mod extractor;
pub mod html_cleansing;
pub mod pattern;

pub use checker::*;
pub use cleaners::{
    clean_text, convert_full2half, extract_wechat_id, remove_email, remove_email_with_prefix,
    remove_exception_char, remove_id_card, remove_ip_address, remove_parentheses,
    remove_phone_number, remove_phone_number_with_prefix, remove_qq, remove_url,
    remove_url_with_prefix, replace_chinese, replace_email, replace_id_card, replace_ip_address,
    replace_parentheses, replace_phone_number, replace_qq, replace_url,
};
pub use extractor::*;
pub use html_cleansing::{
    clean_html, extract_meta_info, remove_html_tag, remove_menu_div_tag, remove_redundant_char,
};
