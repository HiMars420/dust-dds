use crate::structure::CacheChange;

pub trait StatelessReaderListener: 'static {
    fn on_add_change(&self, _cc: &CacheChange) -> (){}
}

pub struct NoOpStatelessReaderListener;
impl StatelessReaderListener for NoOpStatelessReaderListener {}