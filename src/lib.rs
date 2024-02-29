use std::{collections::hash_map::DefaultHasher, fs::File, hash::{Hash, Hasher}, io::{self, BufReader}, ops::{Deref, DerefMut}, path::PathBuf};

use thiserror::Error;
use serde::{Serialize, Deserialize};

#[derive(Error, Debug)]
pub enum DataReadError{
    #[error("read error")]
    ReadError(#[from] io::Error),
    #[error("serde error error")]
    SerdeError(#[from] serde_json::Error)
}



/// listener struct that runs a method when a change was detected, changes are detected using a hash. 
/// If a value change does not result in a change in the hash the method will not be ran 
///
/// ## example
///
/// ```rust
/// use save_on_write::Listener;
/// #[derive(Hash, Debug)]
/// struct Person {
///     name: String,
///     age: u8
/// }
/// let mut listener = Listener::new(Person{name: "Joe".to_string(), age: 25}, Box::new(|_|{}));
/// {
///     //making a value mutable does not mean it will detect a change
///     let mut lck = listener.lock();
///     let a = lck.age;
///     assert!(lck.detected_change() == false);
/// }
/// {
///     let mut lck = listener.lock();
///     lck.age = 20;
///     assert!(lck.detected_change() == true);
/// }
/// ```
pub struct HashListener<T: Hash>
{
    item: T,
    method: Box<dyn FnMut(&mut T)>
}

/// a lock used to detect if a value was changed on drop
pub struct HashListenerLock<'a, T: Hash>
{
    listener: &'a mut HashListener<T>,
    possible_change: bool,
    hash: u64
}
impl<T: Hash> HashListener<T>{
    pub fn new(item: T, method: Box<dyn FnMut(&mut T)>) -> Self {
        Self {
            item,
            method
        }
    }
    pub fn lock(& mut self) -> HashListenerLock<'_, T>{
        let mut hasher = DefaultHasher::new();
        self.item.hash(&mut hasher);
        HashListenerLock {
            listener: self,
            possible_change: false,
            hash: hasher.finish()
        }
    }
}

impl<'a, T: Hash> HashListenerLock<'a, T>{
    #[allow(unused)]
    pub(crate) fn detected_possible_change(&self) -> bool {
        self.possible_change
    }
    pub fn detected_change(&self) -> bool {
        if !self.possible_change {
            return false;
        }
        let mut hasher = DefaultHasher::new();
        self.listener.item.hash(&mut hasher);
        let hash2 = hasher.finish();
        
        self.hash != hash2
    }
}

impl<'a, T: Hash> Deref for HashListenerLock<'a, T>{
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.listener.item
    }
}
impl<'a, T: Hash> DerefMut for HashListenerLock<'a, T>{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.possible_change = true;
        &mut self.listener.item
    }
}

impl<'a, T: Hash> Drop for HashListenerLock<'a, T>{
    fn drop(&mut self) {
        if !self.detected_change(){
            return;
        }

        eprintln!("detected change");
        let method = &mut self.listener.method;
        method(&mut self.listener.item)
    }
}
/// write on save
pub struct SoW<T: Hash>{
    item: HashListener<T>,
}
impl<T> SoW<T>
where
    T: Hash + Serialize,
    T: for<'a> Deserialize<'a>,
{
    pub fn new_from_file(file: PathBuf) -> Result<Self, DataReadError> {
        let f = File::open(file.clone())?;
        let reader = BufReader::new(f);

        let data: T = serde_json::from_reader(reader)?;
        let pth = file.clone();
        let a = move |item: &mut T | {
            let _ = std::fs::write(&pth, serde_json::to_string(&item).unwrap());
        };
        let item = HashListener::new(data, Box::new(a));
        Ok(
            Self {
                item
            }
        )
    }
    pub fn new_from_item(item: T, dest: PathBuf) -> Result<Self, DataReadError>{
        let text = serde_json::to_string(&item)?;
        std::fs::write(&dest, text)?;
        let pth = dest.clone();
        let a = move |item: &mut T | {
            let _ = std::fs::write(&pth, serde_json::to_string(&item).unwrap());
        };
        let item = HashListener::new(item, Box::new(a));
        Ok(
            Self {
                item
            }
        )
    }
}

impl<T: Hash> Deref for SoW<T>{
    type Target = HashListener<T>;
    fn deref(&self) -> &Self::Target {
        &self.item
    }
}
impl<T: Hash> DerefMut for SoW<T>{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.item
    }
}

// TODO: implement a clone listener

#[cfg(test)]
mod tests{
    use crate::HashListener;

    #[test]
    pub fn has_changed(){
        let a = 5;
        let mut notifier = HashListener::new(a, Box::new(|b| {}));
        {
            #[allow(unused_mut)]
            let mut b = notifier.lock();
            let _ = *b + 5;
            assert!(b.detected_change() == false);
        }
    }
}