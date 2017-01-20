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
// use chan::Receiver;

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

fn do_it(root: &Path) -> io::Result<()> {
    let jobs = {
        let (s, r) = chan::sync(0);
        let pb = root.to_path_buf();
        thread::spawn(move || {
            for e in WalkDir::new(pb) {
                let e = match e {
                    Ok(e) => e,
                    Err(_) => continue, //? TODO: send Result with error
                };
                if !e.file_type().is_file() {
                    continue;
                }
                s.send(e);
            }
        });
        // This extra lexical scope will drop the initial
        // sender we created. Thus, the channel will be
        // closed when all threads spawned above has completed.
        r
    };
    let wg = chan::WaitGroup::new();
    for _ in 0..8 {
        wg.add(1);
        let wg = wg.clone();
        let jobs = jobs.clone();
        thread::spawn(move || {
            let mut hasher = Sha1::new();
            let mut buf = [0; 1024 * 8];
            for e in jobs {
                let hex = match calc_hash(e.path(), &mut hasher, &mut buf) {
                    Ok(h) => h,
                    Err(_) => continue, //? TODO: Handle error
                };
                println!("{}\t{}", hex, e.path().display());
            }
            wg.done();
        });
    }
    wg.wait();
    Ok(())
}

fn process_dir_entry(e: walkdir::Result<walkdir::DirEntry>) -> i32 {
    let e = match e {
        Ok(e) => e,
        Err(_) => return 0, //? TODO: send Result with error
    };
    if !e.file_type().is_file() {
        return 0;
    }
    1
}

/// factory is needed to allow iterators missing std::marker::Send
/// TODO: spawn worker threads
/// TODO: use channels
/// TODO: return channel Receiver as Iterator
fn fan_out_in<I, F, T, W>(factory: F, worker: W) -> i32
    where F: 'static + std::marker::Send + FnOnce() -> Box<T>,
          W: 'static + std::marker::Send + Fn(I) -> i32,
          T: IntoIterator<Item = I>
{
    let t = thread::spawn(move || {
        let mut i = 0;
        let it = factory();
        for e in it.into_iter() {
            i += worker(e);
        }
        i
    });
    t.join().unwrap()
}

fn do_it_2(root: &Path) -> io::Result<()> {
    let pb = root.to_path_buf();
    let iterator_factory = || Box::new(WalkDir::new(pb));
    let i = fan_out_in(iterator_factory, process_dir_entry);
    println!("{}", i);
    Ok(())
}

fn main() {
    let root = env::args().nth(1).unwrap_or(".".to_string());
    let root = Path::new(root.as_str());
    do_it(root).expect("Hello error 1!");
    do_it_2(root).expect("Hello error 2!");
}
