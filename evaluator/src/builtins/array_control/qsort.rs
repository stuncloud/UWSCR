use crate::object::Object;

use std::cmp::Ordering;

#[derive(Debug)]
pub struct Qsort {
    order: SortOrder,
}

type SortArrays = [Option<Vec<Object>>; 8];

impl Qsort {
    pub fn new(order: SortOrder) -> Self {
        Self { order }
    }

    fn compare(&self, obj1: &Object, obj2: &Object) -> Option<Ordering> {
        match self.order {
            SortOrder::Ascending => {
                obj1.partial_cmp(obj2)
            },
            SortOrder::Descending => {
                obj2.partial_cmp(obj1)
            },
            SortOrder::UnicodeAsc => {
                let a = obj1.to_string();
                let b = obj2.to_string();
                a.encode_utf16().partial_cmp(b.encode_utf16())
            },
            SortOrder::UnicodeDsc => {
                let a = obj1.to_string();
                let b = obj2.to_string();
                b.encode_utf16().partial_cmp(a.encode_utf16())
            },
            SortOrder::NaturalAsc => {
                let a = obj1.to_string().parse::<f64>();
                let b = obj2.to_string().parse::<f64>();
                if let Ok(a) = a {
                    if let Ok(b) = b {
                        a.partial_cmp(&b)
                    } else {
                        Some(Ordering::Less)
                    }
                } else {
                    if let Ok(_) = b {
                        Some(Ordering::Greater)
                    } else {
                        obj1.partial_cmp(obj2)
                    }
                }
            },
            SortOrder::NaturalDsc => {
                let a = obj1.to_string().parse::<f64>();
                let b = obj2.to_string().parse::<f64>();
                if let Ok(a) = a {
                    if let Ok(b) = b {
                        b.partial_cmp(&a)
                    } else {
                        Some(Ordering::Greater)
                    }
                } else {
                    if let Ok(_) = b {
                        Some(Ordering::Less)
                    } else {
                        obj2.partial_cmp(obj1)
                    }
                }
            },
        }
    }

    pub fn sort(&self, array: &mut [Object], arrays: &mut SortArrays) {
        let len = array.len();
        arrays.iter_mut()
            .for_each(|v| {
                if let Some(a) = v {
                    if a.len() < len {
                        a.resize(len, Object::Empty);
                    }
                }
            });
        let from = 0;
        let to = (len - 1) as isize;
        self.sort_partition(array, from, to, arrays);

    }
    fn partition(&self, array: &mut [Object], l: isize, h: isize, arrays: &mut SortArrays) -> isize {
        // let pivot = array[h as usize].clone();
        let mut i = l - 1;
        for j in l..h {
            if let Some(ordering) = self.compare(&array[h as usize], &array[j as usize]) {
                match ordering {
                    Ordering::Greater |
                    Ordering::Equal => {
                        i += 1;
                        let a = i as usize;
                        let b = j as usize;
                        array.swap(a, b);
                        for arr in arrays.as_mut() {
                            if let Some(arr) = arr.as_mut() {
                               arr.swap(a, b);
                            }
                        }
                    },
                    _ => {},
                }
            }
        }
        i += 1;
        let a = i as usize;
        let b = h as usize;
        array.swap(a, b);
        for arr in arrays.as_mut() {
            if let Some(arr) = arr.as_mut() {
                arr.swap(a, b);
            }
        }
        i
    }
    fn sort_partition(&self, array: &mut [Object], from: isize, to: isize, arrays: &mut SortArrays) {
        if from < to && to - from >= 1 {
            let p = self.partition(array, from, to, arrays);
            self.sort_partition(array, from, p - 1, arrays);
            self.sort_partition(array, p + 1, to, arrays);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortOrder {
    Ascending,
    Descending,
    UnicodeAsc,
    UnicodeDsc,
    NaturalAsc,
    NaturalDsc,
}
