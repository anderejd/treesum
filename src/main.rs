#![deny(warnings)]

extern crate crypto;
extern crate sgiter;
extern crate walkdir;

use self::crypto::digest::Digest;
use self::crypto::sha1::Sha1;
use std::env;
use std::fs::File;
use std::io::Read;
use std::io;
use std::path::Path;
use walkdir::DirEntry;
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

#[derive(Debug)]
enum Error {
    Io(io::Error),
    Ignored(DirEntry),
    WalkDir(walkdir::Error),
}

fn process_root(root: &Path) -> io::Result<()> {
    use sgiter::scatter_gather;
    let pb = root.to_path_buf();
    let producer_ctor = || WalkDir::new(pb);
    let xform_ctor = || {
        let mut hasher = Sha1::new();
        let mut buf = [0; 1024 * 8];
        let f = move |e: walkdir::Result<DirEntry>| {
            let e = e.map_err(Error::WalkDir)?;
            if !e.file_type().is_file() {
                return Err(Error::Ignored(e));
            }
            calc_hash(e.path(), &mut hasher, &mut buf)
                .map_err(Error::Io)
                .map(|s| (e, s))
        };
        f
    };
    let results = scatter_gather(producer_ctor, xform_ctor);
    //let sort_on_hash = true;
    let mut tuples = vec![];
    for r in results {
        match r {
            Ok(t) => tuples.push(t),
            Err(e) => print_err(e),
        }
    }
    tuples.sort_by(|a, b| a.1.cmp(&b.1));
    for t in tuples {
        println!("{}\t{}", t.1, t.0.path().display());
    }
    Ok(())
}

fn print_err(e: Error) {
    let verbose = true;
    match e {
        Error::Ignored(ent) => {
            if verbose {
                println!("ignored: {}", ent.path().display())
            }
        }
        _ => println!("ERROR: {:?}", e),
    }
}

fn main() {
    let root = env::args().nth(1).unwrap_or(".".to_string());
    let root = Path::new(root.as_str());
    process_root(root).unwrap()
}
