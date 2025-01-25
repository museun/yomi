use mlua::{FromLua, IntoLua};

#[derive(Copy, Clone)]
pub struct Globals<'a>(pub(crate) &'a mlua::Lua);

impl<'a> Globals<'a> {
    pub const fn new(lua: &'a mlua::Lua) -> Self {
        Self(lua)
    }

    pub fn set(&self, key: &str, value: impl IntoLua) -> Result<(), mlua::Error> {
        self.0.globals().set(key, value)
    }

    pub fn get<T: FromLua>(&self, key: &str) -> Result<T, mlua::Error> {
        self.0.globals().get(key)
    }
}

impl<'a> Globals<'a> {
    pub fn register(self, item: impl GlobalItem) -> mlua::Result<Self> {
        item.register(self).map(|_| self)
    }
}

pub trait GlobalItem: IntoLua {
    const MODULE: &'static str;
    fn register(self, g: Globals<'_>) -> mlua::Result<()> {
        g.set(Self::MODULE, self)
    }
}

impl<T: GlobalItem + Clone> GlobalItem for &T
where
    for<'a> &'a T: IntoLua,
{
    const MODULE: &'static str = <T as GlobalItem>::MODULE;
    fn register(self, g: Globals<'_>) -> mlua::Result<()> {
        g.set(Self::MODULE, self.clone())
    }
}
