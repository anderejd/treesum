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
    use std::thread;
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
            let mut buf = [0; 4096];
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

fn main() {
    let root = env::args().nth(1).unwrap_or(".".to_string());
    let root = Path::new(root.as_str());
    do_it(root).expect("Hello error!");
}
