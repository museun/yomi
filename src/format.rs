pub trait FormatTime {
    fn as_readable_time(&self) -> String;
    fn as_fuzzy_time(&self) -> String {
        self.as_readable_time()
    }
}

impl FormatTime for std::time::Duration {
    fn as_readable_time(&self) -> String {
        format_seconds(self.as_secs_f64() as _)
    }
}

impl FormatTime for time::Duration {
    fn as_readable_time(&self) -> String {
        format_seconds(self.as_seconds_f64() as _)
    }

    fn as_fuzzy_time(&self) -> String {
        fuzzy_seconds(self.as_seconds_f64() as _)
    }
}

fn fuzzy_seconds(secs: u64) -> String {
    let d = ::time::Duration::new(secs as _, 0);
    macro_rules! maybe {
        ($($expr:tt => $class:expr)*) => {{
            $(
                match d.$expr() {
                    0 => {}
                    1 => return format!("1 {} ago", $class),
                    d => return format!("{d} {}s ago", $class),
                }
            )*
            String::from("just now")
        }};
    }

    match d.whole_weeks() {
        n if n < 52 => {}
        52 => return String::from("a year ago"),
        n => return format!("{n} years ago", n = n / 52),
    }

    maybe! {
        whole_weeks   => "week"
        whole_days    => "day"
        whole_hours   => "hour"
        whole_minutes => "minute"
        whole_seconds => "second"
    }
}

fn format_seconds(mut secs: u64) -> String {
    const TABLE: [(&str, u64); 4] = [
        ("days", 86400), //
        ("hours", 3600), //
        ("minutes", 60), //
        ("seconds", 1),  //
    ];

    fn plural(s: &str, n: u64) -> String {
        format!("{n} {}", if n > 1 { s } else { &s[..s.len() - 1] })
    }

    let mut time = vec![];
    for (name, d) in TABLE {
        let div = secs / d;
        if div > 0 {
            time.push(plural(name, div));
            secs -= d * div
        }
    }

    let len = time.len();
    if len > 1 {
        if len > 2 {
            for segment in time.iter_mut().take(len - 2) {
                segment.push(',');
            }
        }
        time.insert(len - 1, "and".into());
    };

    time.join(" ")
}
