use std::collections::hash_map::DefaultHasher;
use std::collections::VecDeque;
use std::fs::{read, read_dir};
use std::hash::Hash;
use std::hash::Hasher;
use std::io::Error;
use std::path::PathBuf;

use rayon::prelude::*;

fn main() {
    // read first string from stdin
    let path = std::env::args().nth(1).expect("Usage: treehash [path]");
    let tree = Tree::<IO>::from_dir(PathBuf::from(path)).unwrap();
    println!("{:?}", tree.hash());
}

pub struct Tree<T> {
    value: T,
    children: Vec<Self>,
}

pub struct Nodes<'a, T> {
    queue: VecDeque<&'a Tree<T>>,
}

impl<'a, T> Iterator for Nodes<'a, T> {
    type Item = &'a Tree<T>;

    fn next(&mut self) -> Option<Self::Item> {
        let node = self.queue.pop_front()?;
        self.queue.extend(&node.children);
        Some(node)
    }
}

impl<T> Tree<T> {
    fn nodes(&self) -> impl Iterator<Item = &Self> {
        Nodes {
            queue: VecDeque::from([self]),
        }
    }

    pub fn values(&self) -> impl Iterator<Item = &T> {
        self.nodes().map(|n| &n.value)
    }
}

// you could do an inline closure but I felt like moving it out here
fn hash_val<T: Hash>(val: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    val.hash(&mut hasher);
    hasher.finish()
}

#[derive(Debug)]
pub enum IO {
    File(PathBuf),
    Dir(PathBuf),
}

impl Hash for IO {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            IO::File(path) => {
                path.hash(state);
                let contents = read(path).unwrap();
                contents.hash(state);
            }
            IO::Dir(path) => {
                path.hash(state);
            }
        }
    }
}

impl<T: Hash + Sync> Tree<T> {
    pub fn hash(&self) -> u64 {
        // hash the "shape" of the tree
        // there might still be something not quite right with this,
        // like needing to also include the node count, idk
        let mut hasher = DefaultHasher::new();
        for node in self.nodes() {
            hasher.write_usize(node.children.len());
        }
        let shape_hash = hasher.finish();

        // whatever. this is probably way cheaper than the "hard part"
        // and I've never implemented a rayon ParallelIterator before
        let values: Vec<&T> = self.values().collect();

        // it's rayon time, baby
        let val_hash = values
            .into_par_iter()
            .map(hash_val)
            .reduce(|| 0, |a, b| a ^ b);

        shape_hash ^ val_hash
    }
    // Ignores symlinks
    pub fn from_dir(path: PathBuf) -> Result<Tree<IO>, Error> {
        let dir = read_dir(&path)?;
        let mut children: Vec<Tree<IO>> = Vec::new();
        for entry in dir {
            let entry = entry?;
            let t = entry.file_type()?;
            if t.is_file() {
                let val = IO::File(entry.path());
                children.push(Tree {
                    value: val,
                    children: Vec::new(),
                });
            }
            if t.is_dir() {
                let tree = Tree::<T>::from_dir(entry.path())?;
                children.push(tree);
            }
        }

        Ok(Tree {
            value: IO::Dir(path),
            children,
        })
    }
}
