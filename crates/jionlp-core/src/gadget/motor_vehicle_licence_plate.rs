//! Motor vehicle licence plate parser — port of
//! `jionlp/gadget/motor_vehicle_licence_plate.py`.
//!
//! Accepts mainland plates in the 92-format (7 chars: `京A12345`) and
//! new-energy format (8 chars: `京A12345D`) — with or without the cosmetic
//! separator `·` / `.` / ` ` / `　` (fullwidth) between the provincial
//! abbreviation and the serial.
//!
//! Returns car_loc (first two chars), car_type (GV / PEV / NPEV) and
//! car_size (Some for new-energy: small/big; None for 92-format).

use crate::rule::pattern::{
    MOTOR_VEHICLE_LICENCE_PLATE_CHECK_PATTERN, NEV_BIG_PATTERN, NEV_SMALL_PATTERN,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlateInfo {
    /// First two chars, e.g. "川A".
    pub car_loc: String,
    /// "GV" (gasoline), "PEV" (pure electric), "NPEV" (non-pure EV).
    pub car_type: String,
    /// Some("small") or Some("big") for new-energy plates; None for 92-format.
    pub car_size: Option<&'static str>,
}

const GAP_CHARS: &[char] = &['·', '.', ' ', '\u{3000}'];

/// PEV / NPEV mapping for the 5th char of new-energy plates (for small) or
/// the 8th char (for big).
fn classify_nev_letter(c: char) -> Option<&'static str> {
    match c {
        'A' | 'B' | 'C' | 'D' | 'E' => Some("PEV"),
        'F' | 'G' | 'H' | 'J' | 'K' => Some("NPEV"),
        _ => None,
    }
}

/// Parse a licence plate. Returns `None` if the input doesn't look like a
/// valid mainland plate.
pub fn parse_motor_vehicle_licence_plate(plate: &str) -> Option<PlateInfo> {
    if !MOTOR_VEHICLE_LICENCE_PLATE_CHECK_PATTERN.is_match(plate) {
        return None;
    }

    let chars: Vec<char> = plate.chars().collect();
    let len = chars.len();
    let car_loc: String = chars.iter().take(2).collect();

    match len {
        9 => parse_nev(plate, &chars, car_loc),
        8 => {
            // Either 92-format-with-separator (e.g. "川A·23047") or a NEV
            // written tightly without separator ("川A23047B").
            if GAP_CHARS.contains(&chars[2]) {
                Some(PlateInfo {
                    car_loc,
                    car_type: "GV".to_string(),
                    car_size: None,
                })
            } else {
                parse_nev(plate, &chars, car_loc)
            }
        }
        7 => Some(PlateInfo {
            car_loc,
            car_type: "GV".to_string(),
            car_size: None,
        }),
        _ => None,
    }
}

fn parse_nev(plate: &str, _chars: &[char], car_loc: String) -> Option<PlateInfo> {
    let small = NEV_SMALL_PATTERN.find(plate);
    let big = NEV_BIG_PATTERN.find(plate);
    match (small, big) {
        (Some(s), None) => {
            let letter = s.as_str().chars().next()?;
            Some(PlateInfo {
                car_loc,
                car_type: classify_nev_letter(letter)?.to_string(),
                car_size: Some("small"),
            })
        }
        (None, Some(b)) => {
            let letter = b.as_str().chars().last()?;
            Some(PlateInfo {
                car_loc,
                car_type: classify_nev_letter(letter)?.to_string(),
                car_size: Some("big"),
            })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gv_with_separator() {
        let r = parse_motor_vehicle_licence_plate("川A·23047").unwrap();
        assert_eq!(r.car_loc, "川A");
        assert_eq!(r.car_type, "GV");
        assert_eq!(r.car_size, None);
    }

    #[test]
    fn gv_no_separator_7_chars() {
        let r = parse_motor_vehicle_licence_plate("京A12345").unwrap();
        assert_eq!(r.car_loc, "京A");
        assert_eq!(r.car_type, "GV");
    }

    #[test]
    fn nev_big_with_separator() {
        // "23047B" = 5 digits + letter → big new-energy. Python docstring
        // lists this as car_size='big', car_type='PEV' (B → PEV).
        let r = parse_motor_vehicle_licence_plate("川A·23047B").unwrap();
        assert_eq!(r.car_loc, "川A");
        assert_eq!(r.car_type, "PEV");
        assert_eq!(r.car_size, Some("big"));
    }

    #[test]
    fn nev_small_with_separator() {
        // "D12345" = letter + 5 digits → small new-energy.
        let r = parse_motor_vehicle_licence_plate("京A·D12345").unwrap();
        assert_eq!(r.car_loc, "京A");
        assert_eq!(r.car_type, "PEV"); // D → PEV
        assert_eq!(r.car_size, Some("small"));
    }

    #[test]
    fn nev_big_npev() {
        // F is NPEV letter.
        let r = parse_motor_vehicle_licence_plate("京A·12345F").unwrap();
        assert_eq!(r.car_type, "NPEV");
        assert_eq!(r.car_size, Some("big"));
    }

    #[test]
    fn invalid_plate_rejected() {
        assert!(parse_motor_vehicle_licence_plate("XY·1234").is_none());
        assert!(parse_motor_vehicle_licence_plate("not a plate").is_none());
    }

    #[test]
    fn excluded_regions_rejected() {
        // 港澳台 are not mainland plates and should be rejected by this parser.
        assert!(parse_motor_vehicle_licence_plate("港A12345").is_none());
    }
}
