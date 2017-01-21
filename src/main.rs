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

fn process_root(root: &Path) -> io::Result<()> {
    let pb = root.to_path_buf();
    let producer_ctor = || WalkDir::new(pb);
    let xform_ctor = || {
        let mut hasher = Sha1::new();
        let mut buf = [0; 1024 * 8];
        let f = move |e: walkdir::Result<DirEntry>| -> Result<String> {
            let e = e.map_err(TreesumError::WalkDir)?;
            if !e.file_type().is_file() {
                return Err(TreesumError::Ignored(e));
            }
            calc_hash(e.path(), &mut hasher, &mut buf).map_err(TreesumError::Io)
        };
        f
    };
    let results = scatter_gather(producer_ctor, xform_ctor);
    for r in results {
        println!("{:?}", r);
    }
    Ok(())
}

fn main() {
    let root = env::args().nth(1).unwrap_or(".".to_string());
    let root = Path::new(root.as_str());
    process_root(root).expect("Hello error 2!");
}

/// The purpose of this newtype is to hide the channel implementation.
struct GatherIter<T>(chan::Iter<T>);
impl<T> Iterator for GatherIter<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        self.0.next()
    }
}

/// producer_ctor and xform_xtor is needed to allow construction
/// in the correct producer / worker thread.
fn scatter_gather<PC, XC, P, X, J, R>(producer_ctor: PC, xform_ctor: XC) -> GatherIter<R>
    where PC: 'static + std::marker::Send + FnOnce() -> P,
          XC: 'static + std::marker::Send + std::marker::Sync + Fn() -> X,
          X: FnMut(J) -> R,
          J: 'static + std::marker::Send,
          R: 'static + std::marker::Send,
          P: IntoIterator<Item = J>
{
    let jobs_rx = {
        let (tx, rx) = chan::sync(0);
        thread::spawn(move || {
            for e in producer_ctor().into_iter() {
                tx.send(e);
            }
        });
        rx
    };
    let results_rx = {
        let (tx, rx) = chan::sync(0);
        let xform_ctor = Arc::new(xform_ctor); // TODO: Investigate why this is needed.
        for _ in 0..8 {
            let tx = tx.clone();
            let jobs_rx = jobs_rx.clone();
            let xform_ctor = xform_ctor.clone();
            thread::spawn(move || {
                let mut xform = xform_ctor();
                for e in jobs_rx {
                    tx.send(xform(e));
                }
            });
        }
        rx
    };
    GatherIter(results_rx.iter())
}