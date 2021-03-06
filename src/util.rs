use std::default::Default;

pub fn get_mut_or_set<T, F>(opt: &mut Option<T>, f: F) -> &mut T
    where F: Fn() -> T
{
    if opt.is_none() {
        *opt = Some(f());
    }
    opt.as_mut().unwrap()
}

pub fn set_default<T: Default>(opt: &mut Option<T>) -> &mut T {
    get_mut_or_set(opt, Default::default)
}

pub fn replace<T>(opt: &mut Option<T>, target: Option<T>) {
    let old = opt.take();
    *opt = target.or(old);
}

pub fn merge_vec<T, I>(base: &mut Vec<T>, data: I)
    where T: PartialEq, I: Iterator<Item=T>
{
    for i in data {
        if !base.contains(&i) {
            base.push(i);
        }
    }
}
