use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub enum Part {
    Exact(String),
    Argument(String),
    Optional(String),
    Variadic(String),
}

impl Part {
    fn name(&self) -> &str {
        match self {
            Self::Exact(name)
            | Self::Argument(name)
            | Self::Optional(name)
            | Self::Variadic(name) => name,
        }
    }

    const fn is_exact(&self) -> bool {
        matches!(self, Self::Exact(..))
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum Kind {
    None,
    Optional,
    Variadic,
}

fn lex(input: &mut &str) -> Vec<Part> {
    fn try_merge(out: Option<&mut Part>, current: Part) -> Option<Part> {
        if let Some(last) = out.and_then(|c| {
            let Part::Exact(last) = c else { return None };
            current.is_exact().then_some(last)
        }) {
            last.push(' ');
            last.push_str(current.name());
            None
        } else {
            Some(current)
        }
    }

    let mut out = <Vec<Part>>::new();

    loop {
        match unfold(input) {
            Some(head) => {
                if let Some(part) = classify(head) {
                    let value = try_merge(out.last_mut(), part);
                    out.extend(value);
                }
            }
            None => {
                let value = classify(input).and_then(|part| {
                    try_merge(out.last_mut(), part) //
                });
                out.extend(value);
                *input = "";
                break;
            }
        }
    }

    out
}

fn classify(head: &str) -> Option<Part> {
    fn is_surround(input: &str, head: &str, tail: &str) -> bool {
        input.starts_with(head) && input.ends_with(tail)
    }

    let (n, kind) = match () {
        _ if head.trim().is_empty() => return None,
        _ if is_surround(head, "<", "...>") => (4, Kind::Variadic),
        _ if is_surround(head, "<", "?>") => (2, Kind::Optional),
        _ if is_surround(head, "<", ">") => (1, Kind::None),
        _ => return Some(Part::Exact(head.to_string())),
    };
    classify_part(head, n, kind)
}

fn classify_part(arg: &str, tail: usize, kind: Kind) -> Option<Part> {
    if arg.contains(char::is_whitespace) {
        return Some(Part::Exact(arg.to_string()));
    }
    let arg = &arg[1..arg.len() - tail];
    if arg.is_empty() {
        return None;
    }
    Some(match kind {
        Kind::None => Part::Argument(arg.to_string()),
        Kind::Optional => Part::Optional(arg.to_string()),
        Kind::Variadic => Part::Variadic(arg.to_string()),
    })
}

fn unfold<'a>(input: &mut &'a str) -> Option<&'a str> {
    if input.starts_with('<') {
        if let Some(next) = input.find('>').map(|d| d + 1) {
            if input[..next].chars().any(char::is_whitespace) {
                let tail = input[next..].find(char::is_whitespace)? + next;
                let head = &input[..tail];
                *input = &input[tail..];
                return Some(head);
            }
        }
    }

    let (head, tail) = input.split_once(' ')?;
    *input = tail;
    Some(head)
}

#[derive(Debug, thiserror::Error)]
pub enum PatternError {
    #[error("Duplicate name: {name}")]
    DuplicateName { name: String },

    #[error("Ambigious variadic '{variadic}' may overlap with '{pattern}'")]
    AmbigiousVariadic { variadic: String, pattern: String },

    #[error("Ambigious optional '{optional}' may overlap with '{pattern}'")]
    AmbigiousOptional { optional: String, pattern: String },
}

#[derive(Debug)]
pub enum Pattern {
    Exact(String),
    Arguments(Vec<Part>),
}

impl Pattern {
    pub fn parse(mut input: &str) -> Result<Self, PatternError> {
        let mut args = lex(&mut input);
        let [Part::Exact(ref mut part)] = &mut args[..] else {
            #[derive(Copy, Clone, Default, Debug, PartialEq)]
            enum State {
                #[default]
                None,
                Argument,
                Optional,
                Variadic,
            }
            let mut seen = HashSet::new();
            let mut state = State::default();
            let mut prev = <Option<&str>>::None;

            for part in &args {
                match part {
                    Part::Exact(_) => state = State::None,
                    Part::Argument(binding) => {
                        if !seen.insert(binding) {
                            return Err(PatternError::DuplicateName {
                                name: binding.to_string(),
                            });
                        }

                        match state {
                            State::Optional => {
                                return Err(PatternError::AmbigiousOptional {
                                    pattern: binding.to_string(),
                                    optional: prev.take().unwrap().to_string(),
                                });
                            }
                            State::Variadic => {
                                return Err(PatternError::AmbigiousVariadic {
                                    pattern: binding.to_string(),
                                    variadic: prev.take().unwrap().to_string(),
                                });
                            }
                            _ => {}
                        }

                        state = State::Argument;
                        prev = Some(binding);
                    }
                    Part::Optional(binding) => {
                        if !seen.insert(binding) {
                            return Err(PatternError::DuplicateName {
                                name: binding.to_string(),
                            });
                        }
                        if !matches!(state, State::None) {
                            return Err(PatternError::AmbigiousOptional {
                                pattern: binding.to_string(),
                                optional: prev.take().unwrap().to_string(),
                            });
                        }
                        state = State::Optional;
                        prev = Some(binding)
                    }
                    Part::Variadic(binding) => {
                        if !seen.insert(binding) {
                            return Err(PatternError::DuplicateName {
                                name: binding.to_string(),
                            });
                        }

                        if !matches!(state, State::None | State::Argument) {
                            return Err(PatternError::AmbigiousVariadic {
                                variadic: prev.take().unwrap().to_string(),
                                pattern: binding.to_string(),
                            });
                        }

                        state = State::Variadic;
                        prev = Some(binding)
                    }
                }
            }
            return Ok(Self::Arguments(args));
        };

        Ok(Self::Exact(std::mem::take(part)))
    }

    pub fn extract<'a, 'b>(&'a self, mut input: &'b str) -> Extract<'a, 'b> {
        let args = match self {
            Self::Exact(data) if data == input => return Extract::Match,
            Self::Exact(..) => return Extract::NoMatch,
            Self::Arguments(args) => args,
        };

        let data = &mut input;
        let mut map = HashMap::default();

        let mut iter = args.iter().peekable();
        while let Some(arg) = iter.next() {
            match arg {
                Part::Exact(pat) => {
                    let Some(tail) = data.strip_prefix(&**pat) else {
                        return Extract::NoMatch;
                    };
                    *data = tail.trim();
                }
                Part::Argument(pat) => {
                    let Some(value) = data.split_terminator(' ').next() else {
                        return Extract::NoMatch;
                    };
                    map.insert(&**pat, Value::String(value));
                    *data = data
                        .strip_prefix(value)
                        .expect("strip value from pat::arg")
                        .trim();
                }
                Part::Optional(pat) => {
                    if let Some(value) = data.split_terminator(' ').next() {
                        map.insert(&**pat, Value::String(value));
                        *data = data
                            .strip_prefix(value)
                            .expect("strip value from pat::arg")
                            .trim()
                    }
                }
                Part::Variadic(pat) => {
                    let next = iter.peek().map(|c| c.name());

                    let mut out = vec![];
                    for part in data.split_terminator(' ').filter(|s| !s.is_empty()) {
                        if Some(part) == next {
                            break;
                        }
                        out.push(part);
                    }

                    let offset = out
                        .iter()
                        .fold(out.len(), |n, t| n + t.len())
                        .min(data.len());
                    *data = data[offset..].trim();
                    map.insert(&**pat, Value::List(out));
                }
            }
        }

        Extract::Bindings { map }
    }

    pub fn is_optional(&self) -> bool {
        match self {
            Self::Exact(_) => false,
            Self::Arguments(vec) => {
                matches!(vec.as_slice(), [Part::Exact(..) | Part::Variadic(..)])
            }
        }
    }
}

#[derive(Debug)]
pub enum Extract<'a, 'b> {
    NoMatch,
    Match,
    Bindings { map: HashMap<&'a str, Value<'b>> },
}

impl Extract<'_, '_> {
    pub fn map_to_lua(map: HashMap<&str, Value<'_>>, lua: &mlua::Lua) -> mlua::Value {
        let table = lua.create_table().unwrap();
        for (k, v) in map {
            match v {
                Value::String(s) => table.set(k, s).unwrap(),
                Value::List(v) => table.set(k, v.as_slice()).unwrap(),
            }
        }
        mlua::Value::Table(table)
    }
}

#[derive(Debug)]
pub enum Value<'a> {
    String(&'a str),
    List(Vec<&'a str>),
}
