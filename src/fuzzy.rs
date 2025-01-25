use mlua::UserData;

use crate::GlobalItem;

pub trait Fuzzy {
    fn as_str(&self) -> &str;
}

impl<T: Fuzzy> Fuzzy for &T {
    fn as_str(&self) -> &str {
        <T as Fuzzy>::as_str(self)
    }
}

impl Fuzzy for str {
    fn as_str(&self) -> &str {
        self
    }
}

impl Fuzzy for String {
    fn as_str(&self) -> &str {
        self
    }
}

pub struct Search;
impl GlobalItem for Search {
    const MODULE: &'static str = "fuzzy";
}

impl UserData for Search {
    fn add_methods<M>(methods: &mut M)
    where
        M: mlua::UserDataMethods<Self>,
    {
        methods.add_function(
            "closest",
            |_lua, (input, data, tolerance): (String, Vec<String>, Option<f32>)| {
                let data = data.iter().collect::<Vec<&String>>();
                let out = self::closest(&input, data, tolerance.unwrap_or(0.5));
                let owned = out.into_iter().map(ToString::to_string);
                let owned = owned.collect::<Vec<_>>();
                Ok(owned)
            },
        );
    }
}

pub fn closest<'a, T: Fuzzy>(
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

// TODO make this optioanlly case sensitive
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

    for (i, l) in left.chars().map(|c| c.to_ascii_lowercase()).enumerate() {
        for (j, r) in right.chars().map(|c| c.to_ascii_lowercase()).enumerate() {
            let cost = if l == r { 0 } else { 1 };
            matrix[i + 1][j + 1] = (matrix[i][j + 1] + 1)
                .min(matrix[i + 1][j] + 1)
                .min(matrix[i][j] + cost)
        }
    }

    matrix[l][r]
}
