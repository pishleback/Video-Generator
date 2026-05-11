use std::rc::Rc;

#[derive(Clone)]
pub enum Timeline<V> {
    Constant(V),
    Function(Rc<dyn Fn(f64) -> V>),
}

impl<V> From<V> for Timeline<V> {
    fn from(v: V) -> Self {
        Timeline::Constant(v)
    }
}

impl<V> Timeline<V> {
    pub fn from_fn(f: impl Fn(f64) -> V + 'static) -> Self {
        Timeline::Function(Rc::new(f))
    }
}

impl<V: Clone> Timeline<V> {
    pub fn at_time(&self, t: f64) -> V {
        match self {
            Timeline::Constant(v) => v.clone(),
            Timeline::Function(g) => g(t),
        }
    }
}
