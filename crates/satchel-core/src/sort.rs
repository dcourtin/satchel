//! Sorting, following docs/spec/01-vues-elements.md §4.
//! Order of criteria: hidden-last (optional) → folders-first (independent of direction) → key.

use crate::entry::Entry;
use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortKey {
    Name,
    Size,
    Modified,
}

/// Natural, case-insensitive comparison: digit runs compare numerically ("file2" < "file10").
/// Base name is compared before the extension ("a.txt" < "a b.txt" would otherwise flip).
/// TODO(spec 01 §4.2): decimal ("1.5" vs "1.10") and version ("1.2.10") nuances.
pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    let (a_base, a_ext) = split_ext(a);
    let (b_base, b_ext) = split_ext(b);
    natural_cmp_raw(a_base, b_base)
        .then_with(|| natural_cmp_raw(a_ext, b_ext))
        // Deterministic tie-break: case-sensitive byte order.
        .then_with(|| a.cmp(b))
}

/// Splits "name.ext" → ("name", "ext"). A leading dot is not an extension; neither is a dot
/// between two digits ("v1.5" has no extension).
fn split_ext(s: &str) -> (&str, &str) {
    match s.rfind('.') {
        Some(0) | None => (s, ""),
        Some(i) => {
            let before = s[..i].chars().next_back();
            let after = s[i + 1..].chars().next();
            if matches!((before, after), (Some(x), Some(y)) if x.is_ascii_digit() && y.is_ascii_digit()) {
                (s, "")
            } else {
                (&s[..i], &s[i + 1..])
            }
        }
    }
}

fn natural_cmp_raw(a: &str, b: &str) -> Ordering {
    let mut ai = a.chars().peekable();
    let mut bi = b.chars().peekable();
    loop {
        match (ai.peek().copied(), bi.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) if x.is_ascii_digit() && y.is_ascii_digit() => {
                let na = take_digits(&mut ai);
                let nb = take_digits(&mut bi);
                // Compare numerically without overflow: strip leading zeros, then length, then digits.
                let ta = na.trim_start_matches('0');
                let tb = nb.trim_start_matches('0');
                let ord = ta.len().cmp(&tb.len()).then_with(|| ta.cmp(tb));
                if ord != Ordering::Equal {
                    return ord;
                }
                // "01" vs "1": fewer leading zeros first.
                let z = na.len().cmp(&nb.len());
                if z != Ordering::Equal {
                    return z;
                }
            }
            (Some(x), Some(y)) => {
                let fx = fold(x);
                let fy = fold(y);
                if fx != fy {
                    return fx.cmp(&fy);
                }
                ai.next();
                bi.next();
            }
        }
    }
}

fn take_digits(it: &mut std::iter::Peekable<std::str::Chars<'_>>) -> String {
    let mut s = String::new();
    while let Some(c) = it.peek().copied() {
        if c.is_ascii_digit() {
            s.push(c);
            it.next();
        } else {
            break;
        }
    }
    s
}

/// Case and (basic Latin-1/Latin Extended-A) accent folding so "é" sorts with "e".
fn fold(c: char) -> char {
    let lower = c.to_lowercase().next().unwrap_or(c);
    match lower {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' => 'a',
        'ç' => 'c',
        'è' | 'é' | 'ê' | 'ë' => 'e',
        'ì' | 'í' | 'î' | 'ï' => 'i',
        'ñ' => 'n',
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' => 'o',
        'ù' | 'ú' | 'û' | 'ü' => 'u',
        'ý' | 'ÿ' => 'y',
        'œ' => 'o',
        'æ' => 'a',
        other => other,
    }
}

/// Sorts in place. `folders_first` stays true in descending order (as in Dolphin).
pub fn sort_entries(entries: &mut [Entry], key: SortKey, ascending: bool, folders_first: bool, hidden_last: bool) {
    entries.sort_by(|a, b| {
        if hidden_last && a.hidden != b.hidden {
            return a.hidden.cmp(&b.hidden);
        }
        if folders_first && a.is_dir != b.is_dir {
            return b.is_dir.cmp(&a.is_dir);
        }
        let ord = match key {
            SortKey::Name => natural_cmp(&a.name, &b.name),
            SortKey::Size => a.size.cmp(&b.size).then_with(|| natural_cmp(&a.name, &b.name)),
            SortKey::Modified => a.modified.cmp(&b.modified).then_with(|| natural_cmp(&a.name, &b.name)),
        };
        if ascending { ord } else { ord.reverse() }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn sorted(mut v: Vec<&str>) -> Vec<&str> {
        v.sort_by(|a, b| natural_cmp(a, b));
        v
    }

    #[test]
    fn digits_compare_numerically() {
        assert_eq!(sorted(vec!["file10", "file2", "file1"]), ["file1", "file2", "file10"]);
        assert_eq!(sorted(vec!["img_100.jpg", "img_20.jpg", "img_3.jpg"]), ["img_3.jpg", "img_20.jpg", "img_100.jpg"]);
    }

    #[test]
    fn case_and_accent_insensitive() {
        assert_eq!(sorted(vec!["b", "A", "a", "C"]), ["A", "a", "b", "C"]);
        assert_eq!(sorted(vec!["zèbre", "été", "ecole"]), ["ecole", "été", "zèbre"]);
    }

    #[test]
    fn base_before_extension() {
        assert_eq!(sorted(vec!["a b.txt", "a.txt"]), ["a.txt", "a b.txt"]);
        assert_eq!(split_ext(".bashrc"), (".bashrc", ""));
        assert_eq!(split_ext("v1.5"), ("v1.5", ""));
        assert_eq!(split_ext("archive.tar.gz"), ("archive.tar", "gz"));
    }

    #[test]
    fn huge_numbers_do_not_overflow() {
        assert_eq!(sorted(vec!["x99999999999999999999999", "x1"]), ["x1", "x99999999999999999999999"]);
    }

    fn e(name: &str, is_dir: bool, size: u64) -> Entry {
        Entry { name: name.into(), path: PathBuf::from(name), is_dir, is_symlink: false, size, modified: None, hidden: name.starts_with('.') }
    }

    #[test]
    fn folders_first_even_descending() {
        let mut v = vec![e("b.txt", false, 1), e("zdir", true, 0), e("a.txt", false, 9), e("adir", true, 0)];
        sort_entries(&mut v, SortKey::Name, false, true, false);
        let names: Vec<_> = v.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["zdir", "adir", "b.txt", "a.txt"]);
    }

    #[test]
    fn hidden_last_wins_over_folders_first() {
        let mut v = vec![e(".cfg", true, 0), e("file", false, 0), e("dir", true, 0)];
        sort_entries(&mut v, SortKey::Name, true, true, true);
        let names: Vec<_> = v.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["dir", "file", ".cfg"]);
    }

    #[test]
    fn size_sort() {
        let mut v = vec![e("big", false, 100), e("small", false, 1)];
        sort_entries(&mut v, SortKey::Size, true, true, false);
        assert_eq!(v[0].name, "small");
    }
}
