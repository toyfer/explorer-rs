//! Natural (human) sort: "file2" < "file10".

use std::cmp::Ordering::{self, Equal, Greater, Less};

pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    let mut ac = a.chars().peekable();
    let mut bc = b.chars().peekable();
    loop {
        match (ac.peek().copied(), bc.peek().copied()) {
            (None, None) => return Equal,
            (None, Some(_)) => return Less,
            (Some(_), None) => return Greater,
            (Some(ca), Some(cb)) => {
                if ca.is_ascii_digit() && cb.is_ascii_digit() {
                    let na = take_u64(&mut ac);
                    let nb = take_u64(&mut bc);
                    match na.cmp(&nb) {
                        Equal => continue,
                        o => return o,
                    }
                } else {
                    let ca = ac.next().unwrap().to_ascii_lowercase();
                    let cb = bc.next().unwrap().to_ascii_lowercase();
                    match ca.cmp(&cb) {
                        Equal => continue,
                        o => return o,
                    }
                }
            }
        }
    }
}

fn take_u64(it: &mut std::iter::Peekable<std::str::Chars<'_>>) -> u64 {
    let mut n: u64 = 0;
    while let Some(c) = it.peek().copied() {
        if c.is_ascii_digit() {
            let _ = it.next();
            n = n.saturating_mul(10).saturating_add((c as u8 - b'0') as u64);
        } else {
            break;
        }
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_in_names() {
        assert_eq!(natural_cmp("file2.txt", "file10.txt"), Less);
        assert_eq!(natural_cmp("file10.txt", "file2.txt"), Greater);
        assert_eq!(natural_cmp("file2.txt", "file2.txt"), Equal);
    }

    #[test]
    fn case_insensitive_letters() {
        assert_eq!(natural_cmp("Abc", "abc"), Equal);
        assert_eq!(natural_cmp("a", "b"), Less);
    }

    #[test]
    fn mixed() {
        assert_eq!(natural_cmp("img2", "img10"), Less);
        assert_eq!(natural_cmp("a100", "a99"), Greater);
    }
}
