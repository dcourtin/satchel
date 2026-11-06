//! Human-readable formatting (French locale for now; i18n later).

/// SI units like KDE's default: 1 Ko = 1000 octets. One decimal under 10 units, comma separator.
pub fn size(bytes: u64) -> String {
    const UNITS: [&str; 6] = ["Ko", "Mo", "Go", "To", "Po", "Eo"];
    if bytes < 1000 {
        return if bytes == 1 { "1 octet".into() } else { format!("{bytes} octets") };
    }
    let mut value = bytes as f64 / 1000.0;
    let mut unit = 0;
    while value >= 1000.0 && unit < UNITS.len() - 1 {
        value /= 1000.0;
        unit += 1;
    }
    let s = if value < 10.0 { format!("{value:.1}") } else { format!("{value:.0}") };
    format!("{} {}", s.replace('.', ","), UNITS[unit])
}

/// "1 469 302" with narrow no-break spaces, as French typography wants.
pub fn grouped(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push('\u{202F}');
        }
        out.push(c);
    }
    out
}

pub fn items(n: u64) -> String {
    if n <= 1 { format!("{n} élément") } else { format!("{n} éléments") }
}

/// Date as "2023-08-29 15:06" in local time.
pub fn date(t: std::time::SystemTime) -> String {
    let secs = match t.duration_since(std::time::UNIX_EPOCH) {
        Ok(d) => d.as_secs() as i64,
        Err(_) => return String::new(),
    };
    let secs = secs + local_offset_secs();
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    format!("{y:04}-{m:02}-{d:02} {:02}:{:02}", rem / 3600, (rem % 3600) / 60)
}

/// Local UTC offset, read once from the TZ database via libc-free heuristic:
/// uses `date +%z` style info from /etc/localtime is complex, so we rely on the `TZ_OFFSET_SECS`
/// override or fall back to parsing `/etc/localtime` lazily. TODO: replace with `jiff`/`time` crate.
fn local_offset_secs() -> i64 {
    use std::sync::OnceLock;
    static OFF: OnceLock<i64> = OnceLock::new();
    *OFF.get_or_init(|| {
        std::process::Command::new("date")
            .arg("+%z")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .and_then(|s| {
                let s = s.trim();
                if s.len() != 5 {
                    return None;
                }
                let sign = if s.starts_with('-') { -1 } else { 1 };
                let h: i64 = s[1..3].parse().ok()?;
                let m: i64 = s[3..5].parse().ok()?;
                Some(sign * (h * 3600 + m * 60))
            })
            .unwrap_or(0)
    })
}

/// Howard Hinnant's days→civil algorithm.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes() {
        assert_eq!(size(0), "0 octets");
        assert_eq!(size(1), "1 octet");
        assert_eq!(size(351), "351 octets");
        assert_eq!(size(1_500), "1,5 Ko");
        assert_eq!(size(9_000_000), "9,0 Mo");
        assert_eq!(size(13_300_000), "13 Mo");
        assert_eq!(size(42_390_000_000), "42 Go");
    }

    #[test]
    fn grouping() {
        assert_eq!(grouped(1_469_302), "1\u{202F}469\u{202F}302");
        assert_eq!(grouped(999), "999");
    }

    #[test]
    fn civil() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(19_600), (2023, 8, 31));
    }
}
