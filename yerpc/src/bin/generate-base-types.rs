
use std::path::PathBuf;
use std::env;
use yerpc::{typescript::export_types_to_file, Message};


fn main() {
    let outpath: PathBuf = env::args().nth(1).expect("Outpath is required").into();
    export_types_to_file::<Message>(&outpath, None).expect("Failed to write TS out");
}
