//! Validators — port of `jionlp/rule/checker.py`.

use super::pattern::*;

/// True if `text` contains at least one Chinese character.
pub fn check_any_chinese_char(text: &str) -> bool {
    ANY_CHINESE_PATTERN.is_match(text)
}

/// True if every character of `text` is Chinese (and `text` is non-empty).
pub fn check_all_chinese_char(text: &str) -> bool {
    !text.is_empty() && WHOLE_CHINESE_PATTERN.is_match(text)
}

/// True if `text` contains at least one ASCII digit.
pub fn check_any_arabic_num(text: &str) -> bool {
    ANY_ARABIC_NUM_PATTERN.is_match(text)
}

/// True if every character of `text` is an ASCII digit (and `text` is non-empty).
pub fn check_all_arabic_num(text: &str) -> bool {
    !text.is_empty() && WHOLE_ARABIC_NUM_PATTERN.is_match(text)
}

/// True if `text` looks like a valid mainland 18-char ID card number by the
/// regex rules (admin-code range + date range + sequence + checksum char).
/// Does NOT verify the ISO 7064 MOD 11-2 checksum — that's a separate concern.
pub fn check_id_card(text: &str) -> bool {
    ID_CARD_CHECK_PATTERN.is_match(text)
}

/// True if `text` looks like a valid mainland cell phone.
pub fn check_cell_phone(text: &str) -> bool {
    CELL_PHONE_CHECK_PATTERN.is_match(text)
}

/// True if `text` looks like a valid mainland motor vehicle plate.
pub fn check_motor_vehicle_licence_plate(text: &str) -> bool {
    MOTOR_VEHICLE_LICENCE_PLATE_CHECK_PATTERN.is_match(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn any_chinese() {
        assert!(check_any_chinese_char("hello 中文"));
        assert!(!check_any_chinese_char("only ascii"));
        assert!(!check_any_chinese_char(""));
    }

    #[test]
    fn all_chinese() {
        assert!(check_all_chinese_char("全部中文"));
        assert!(!check_all_chinese_char("中文 with ascii"));
        assert!(!check_all_chinese_char(""));
    }

    #[test]
    fn any_arabic() {
        assert!(check_any_arabic_num("abc123"));
        assert!(!check_any_arabic_num("abc"));
    }

    #[test]
    fn all_arabic() {
        assert!(check_all_arabic_num("123456"));
        assert!(!check_all_arabic_num("12a3"));
        assert!(!check_all_arabic_num(""));
    }

    #[test]
    fn id_card_check() {
        assert!(check_id_card("11010519900307123X"));
        assert!(!check_id_card("this is not an id"));
        assert!(!check_id_card("99999999999999999X")); // bogus admin code
    }

    #[test]
    fn cell_phone_check() {
        assert!(check_cell_phone("13912345678"));
        assert!(!check_cell_phone("12312345678"));
        assert!(!check_cell_phone("1234567890"));
    }

    #[test]
    fn plate_check() {
        assert!(check_motor_vehicle_licence_plate("川A23047"));
        assert!(check_motor_vehicle_licence_plate("京A·12345"));
        assert!(!check_motor_vehicle_licence_plate("ABC12345"));
    }
}
