use std::cell::{OnceCell, RefCell};
use std::fmt::{Display, Formatter};
use std::ops::Deref;
use std::rc::Rc;

thread_local! {
    static GLOBAL_POOL: OnceCell<StringPool> = const { OnceCell::new() };
    static EMPTY: RefString = RefString(Rc::from("".to_string()));
}

pub fn string_empty() -> RefString {
    EMPTY.with(|c| c.clone())
}

pub fn string_pool_get(k: String) -> RefString {
    GLOBAL_POOL.with(|x| x.get_or_init(StringPool::new).get(k))
}

#[derive(Clone)]
pub struct RefString(Rc<String>);

impl RefString {
    pub fn new(s: String) -> RefString {
        RefString(Rc::new(s))
    }
}

impl Display for RefString {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl Deref for RefString {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

pub struct StringPool {
    pool: RefCell<ahash::HashMap<String, RefString>>,
}

impl StringPool {
    pub fn new() -> Self {
        Self {
            pool: RefCell::new(ahash::HashMap::default()),
        }
    }

    pub fn get(&self, k: String) -> RefString {
        if let Some(r) = self.pool.borrow().get(&k) {
            return RefString(r.0.clone());
        }
        let value = RefString::new(k.clone());
        self.pool.borrow_mut().insert(k, value.clone());
        value
    }
}
