#![experimental]

use std::default::Default;

pub fn get_mut_or_set<T, F>(opt: &mut Option<T>, f: F) -> &mut T
    where F: Fn() -> T
{
    if let Some(v) = opt.as_mut() {
        return v;
    }
    unsafe {
        let opt: *mut Option<T> = opt;
        let opt: &mut Option<T> = opt.as_mut().unwrap();
        *opt = Some(f());
        opt.as_mut().unwrap()
    }
}

pub fn set_default<T: Default>(opt: &mut Option<T>) -> &mut T {
    get_mut_or_set(opt, Default::default)
}
