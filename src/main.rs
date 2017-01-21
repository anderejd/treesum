#![deny(warnings)]

extern crate chan;
extern crate crypto;
extern crate walkdir;

use self::crypto::digest::Digest;
use self::crypto::sha1::Sha1;
use std::env;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::io;
use walkdir::WalkDir;
use std::thread;
use std::sync::Arc;
use walkdir::DirEntry;

fn calc_hash(p: &Path, hasher: &mut Sha1, buf: &mut [u8]) -> io::Result<String> {
    hasher.reset();
    let mut f = File::open(p)?;
    loop {
        let num_read = f.read(buf)?;
        if num_read == 0 {
            break;
        }
        hasher.input(&buf[..num_read]);
    }
    Ok(hasher.result_str())
}

type Result<T> = std::result::Result<T, TreesumError>;

#[derive(Debug)]
enum TreesumError {
    Io(io::Error),
    Ignored(DirEntry),
    WalkDir(walkdir::Error),
}

fn process_dir_entry(e: walkdir::Result<DirEntry>) -> Result<String> {
    let e = e.map_err(TreesumError::WalkDir)?;
    if !e.file_type().is_file() {
        return Err(TreesumError::Ignored(e));
    }
    let mut hasher = Sha1::new();
    let mut buf = [0; 1024 * 8];
    calc_hash(e.path(), &mut hasher, &mut buf).map_err(TreesumError::Io)
}


fn do_it_2(root: &Path) -> io::Result<()> {
    let pb = root.to_path_buf();
    let iter_factory = || WalkDir::new(pb);
    let results = scatter_gather(iter_factory, process_dir_entry);
    for r in results {
        println!("{:?}", r);
    }
    Ok(())
}

fn main() {
    let root = env::args().nth(1).unwrap_or(".".to_string());
    let root = Path::new(root.as_str());
    do_it_2(root).expect("Hello error 2!");
}

/// The purpose of this newtype is to hide the channel implementation.
struct GatherIter<T>(chan::Iter<T>);
impl<T> Iterator for GatherIter<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        self.0.next()
    }
}

/// factory is needed to allow iterators without std::marker::Send
/// TODO: spawn worker threads
/// TODO: use channels
/// TODO: return channel Receiver as Iterator
fn scatter_gather<F, X, J, R, I>(factory: F, xform: X) -> GatherIter<R>
    where F: 'static + std::marker::Send + FnOnce() -> I,
          X: 'static + std::marker::Send + std::marker::Sync + Fn(J) -> R,
          J: 'static + std::marker::Send,
          R: 'static + std::marker::Send,
          I: IntoIterator<Item = J>
{
    let jobs_rx = {
        let (tx, rx) = chan::sync(0);
        thread::spawn(move || {
            for e in factory().into_iter() {
                tx.send(e);
            }
        });
        rx
    };
    let results_rx = {
        let (tx, rx) = chan::sync(0);
        let xform = Arc::new(xform); // TODO: Investigate why this is needed.
        for _ in 0..8 {
            let tx = tx.clone();
            let jobs_rx = jobs_rx.clone();
            let xform = xform.clone();
            thread::spawn(move || {
                for e in jobs_rx {
                    tx.send(xform(e));
                }
            });
        }
        rx
    };
    GatherIter(results_rx.iter())
}