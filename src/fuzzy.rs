use std::borrow::Cow;

use mlua::UserData;

use crate::GlobalItem;

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
            |_lua, (input, data, case_insensitive): (String, Vec<String>, Option<bool>)| {
                let out = self::closest(&input, &data, case_insensitive.unwrap_or(true));
                Ok(out)
            },
        );
    }
}

fn closest(query: &str, choices: &[String], case_insensitive: bool) -> Option<String> {
    fn normalize(s: &str, case_insensitive: bool) -> Cow<'_, str> {
        if case_insensitive {
            Cow::from(s.to_lowercase())
        } else {
            Cow::from(s)
        }
    }

    let query = normalize(query, case_insensitive);

    choices
        .iter()
        .map(|s| {
            let s = normalize(&s, case_insensitive);
            (strsim::jaro_winkler(&query, &s), s)
        })
        .max_by(|(l, _), (r, _)| l.total_cmp(r))
        .filter(|&(score, _)| score > 0.7)
        .map(|(_, s)| s.to_string())
}
