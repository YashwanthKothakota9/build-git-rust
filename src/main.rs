use flate2::read::ZlibDecoder;
#[allow(unused_imports)]
use std::env;
#[allow(unused_imports)]
use std::fs;
use std::fs::File;
use std::io::Read;

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    eprintln!("Logs from your program will appear here!");

    let args: Vec<String> = env::args().collect();
    if args[1] == "init" {
        fs::create_dir(".git").unwrap();
        fs::create_dir(".git/objects").unwrap();
        fs::create_dir(".git/refs").unwrap();
        fs::write(".git/HEAD", "ref: refs/heads/main\n").unwrap();
        println!("Initialized git directory")
    } else if args[1] == "cat-file" {
        let object_hash = args[3].clone();
        let object_path = format!(".git/objects/{}/{}", &object_hash[0..2], &object_hash[2..]);
        let file = File::open(object_path).unwrap();
        let mut decompressed_data = Vec::new();
        let mut zlib = ZlibDecoder::new(file);
        zlib.read_to_end(&mut decompressed_data).unwrap();
        let decompressed_string = String::from_utf8(decompressed_data).unwrap();
        let object_parts: Vec<&str> = decompressed_string.split('\0').collect();
        let object_data = &object_parts[1];
        print!("{}", object_data);
    } else {
        println!("unknown command: {}", args[1])
    }
}
