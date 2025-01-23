// #![cfg_attr(debug_assertions, allow(dead_code, unused_variables,))]
const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

pub mod config;
pub use config::Config;

pub mod bot;
pub mod crates;
pub mod format;
pub mod github;
pub mod helix;
pub mod help;
pub mod irc;
pub mod json;
pub mod pattern;
pub mod rand;
pub mod re;
pub mod spotify;
pub mod store;
pub mod time;

pub mod manifest;
pub use manifest::Manifest;

pub mod watcher;
pub use watcher::Watcher;

pub mod logger;
use logger::Logger;

pub mod fuzzy {
    pub trait Search {
        fn as_str(&self) -> &str;
    }

    impl<T: Search> Search for &T {
        fn as_str(&self) -> &str {
            <T as Search>::as_str(self)
        }
    }

    impl Search for str {
        fn as_str(&self) -> &str {
            self
        }
    }

    impl Search for String {
        fn as_str(&self) -> &str {
            &**self
        }
    }

    pub fn closest<'a, T: Search>(
        input: &str,
        data: impl IntoIterator<Item = T> + 'a,
        tolerance: f32,
    ) -> Vec<T> {
        let mut matches = vec![];
        let mut shortest = f32::INFINITY;
        for s in data {
            let t = s.as_str();
            let distance = distance(input, t);
            let normalized = distance as f32 / input.len().max(t.len()) as f32;
            if normalized < shortest {
                shortest = normalized;
                matches.clear();
                matches.push(s);
            } else if (normalized - shortest) / shortest <= tolerance {
                matches.push(s)
            }
        }
        matches
    }

    fn distance(left: &str, right: &str) -> usize {
        if left == right {
            return 0;
        }

        let l = left.chars().count();
        let r = right.chars().count();
        let mut matrix = vec![vec![0; r + 1]; l + 1];

        for i in 0..=l {
            matrix[i][0] = i;
        }
        for i in 0..=r {
            matrix[0][i] = i;
        }

        for (i, l) in left.chars().enumerate() {
            for (j, r) in right.chars().enumerate() {
                let cost = if l == r { 0 } else { 1 };
                matrix[i + 1][j + 1] = (matrix[i][j + 1] + 1)
                    .min(matrix[i + 1][j] + 1)
                    .min(matrix[i][j] + cost)
            }
        }

        matrix[l][r]
    }
}

pub mod responder;
