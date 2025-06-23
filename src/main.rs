use flate2::read::{ZlibDecoder, ZlibEncoder};
use flate2::Compression;
use sha1::{Digest, Sha1};
#[allow(unused_imports)]
use std::env;
#[allow(unused_imports)]
use std::fs;
use std::fs::File;
use std::io::{Read, Write};

fn main() {
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
    } else if args[1] == "hash-object" {
        let file_name = args[3].clone();
        let mut file = File::open(file_name).unwrap();
        // Read the file into a vector
        let mut file_data = Vec::new();
        file.read_to_end(&mut file_data).unwrap();
        // Create a SHA-1 hash object
        let mut hasher = Sha1::new();
        hasher.update(b"blob ");
        hasher.update(file_data.len().to_string().as_bytes());
        hasher.update(b"\0");
        hasher.update(&file_data);
        let hash = hasher.finalize();
        let hash_string = format!("{:x}", hash);
        // Compress the file data with zlib
        let mut zlib = ZlibEncoder::new(
            std::io::Cursor::new(
                [
                    b"blob ",
                    file_data.len().to_string().as_bytes(),
                    b"\0",
                    &file_data,
                ]
                .concat(),
            ),
            Compression::default(),
        );
        let mut compressed_data = Vec::new();
        zlib.read_to_end(&mut compressed_data).unwrap();
        // Write the compressed data to the .git/objects directory
        let object_path = format!(".git/objects/{}/{}", &hash_string[0..2], &hash_string[2..]);
        fs::create_dir_all(format!(".git/objects/{}", &hash_string[0..2])).unwrap();
        let mut object_file = File::create(object_path).unwrap();
        object_file.write_all(&compressed_data).unwrap();
        // print the hash string
        print!("{}", hash_string);
    } else {
        println!("unknown command: {}", args[1])
    }
}
