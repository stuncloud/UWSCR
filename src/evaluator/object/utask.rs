use super::Object;
use super::super::EvalResult;

use std::fmt;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;


#[derive(Debug, Clone)]
pub struct UTask {
    pub handle: Arc<Mutex<Option<JoinHandle<EvalResult<Object>>>>>,
}

impl fmt::Display for UTask {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let flag = self.handle.lock().unwrap().is_none();
        write!(f, "{}", if flag {"done"} else {"running"})
    }
}
