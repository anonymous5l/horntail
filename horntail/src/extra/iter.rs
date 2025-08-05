use std::path::{Component, Components};

pub(crate) struct ComponentIter<'a> {
    components: Option<Components<'a>>,
}

impl<'a> From<Components<'a>> for ComponentIter<'a> {
    fn from(value: Components<'a>) -> Self {
        Self {
            components: Some(value),
        }
    }
}

impl<'a> Iterator for ComponentIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        let component = self.components.as_mut()?.next()?;
        match component {
            Component::CurDir | Component::ParentDir => {
                let _ = self.components.take();
                None
            }
            Component::Normal(name) => Some(name.to_str().unwrap_or_default()),
            _ => self.next(),
        }
    }
}
