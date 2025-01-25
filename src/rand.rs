use mlua::UserData;

use crate::GlobalItem;

pub struct Rando(fastrand::Rng);

impl GlobalItem for Rando {
    const MODULE: &'static str = "rand";
}

impl Rando {
    pub fn new() -> Self {
        Self(fastrand::Rng::new())
    }

    pub const fn with(rng: fastrand::Rng) -> Self {
        Self(rng)
    }
}

impl UserData for Rando {
    fn add_methods<M>(methods: &mut M)
    where
        M: mlua::UserDataMethods<Self>,
    {
        methods.add_method_mut("shuffle", |lua, this, table: mlua::Table| {
            let mut list = table
                .pairs::<mlua::Integer, mlua::Value>()
                .flatten()
                .collect::<Vec<_>>();

            this.0.shuffle(&mut list);

            for (i, (v, _)) in list.iter_mut().enumerate() {
                *v = mlua::Integer::from((i + 1) as i64);
            }

            lua.create_table_from(list)
        });

        methods.add_method_mut("choose", |_lua, this, table: mlua::Table| {
            let list = table
                .pairs::<mlua::Integer, mlua::Value>()
                .flatten()
                .collect::<Vec<_>>();

            Ok(list.get(this.0.usize(..list.len())).map(|(_, v)| v.clone()))
        });
    }
}
