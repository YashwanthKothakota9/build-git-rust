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
    } else if args[1] == "ls-tree" && args[2] == "--name-only" {
        let tree_hash = args[3].clone();
        let tree_path = format!(".git/objects/{}/{}", &tree_hash[0..2], &tree_hash[2..]);
        let file = File::open(tree_path).unwrap();
        let mut decompressed_data = Vec::new();
        let mut zlib = ZlibDecoder::new(file);
        zlib.read_to_end(&mut decompressed_data).unwrap();

        let null_pos = decompressed_data.iter().position(|&b| b == 0).unwrap();
        let mut entries_data = &decompressed_data[null_pos + 1..];

        while !entries_data.is_empty() {
            let null_pos = entries_data.iter().position(|&b| b == 0).unwrap();
            let mode_name = String::from_utf8(entries_data[0..null_pos].to_vec()).unwrap();
            let parts: Vec<&str> = mode_name.split(' ').collect();
            let filename = parts[1];
            println!("{}", filename);
            entries_data = &entries_data[null_pos + 1 + 20..];
        }
    } else if args[1] == "write-tree" {
        fn write_tree_recursive(dir_path: &std::path::Path) -> Vec<u8> {
            let mut tree_entries = Vec::new();

            // Collect all entries (files and directories)
            for file in fs::read_dir(dir_path).unwrap() {
                let file = file.unwrap();
                let path = file.path();
                let file_name = path.file_name().unwrap().to_str().unwrap();

                // Skip .git directory
                if file_name == ".git" {
                    continue;
                }

                if path.is_file() {
                    // Handle files
                    let file_data = fs::read(&path).unwrap();
                    let mut hasher = Sha1::new();
                    hasher.update(b"blob ");
                    hasher.update(file_data.len().to_string().as_bytes());
                    hasher.update(b"\0");
                    hasher.update(&file_data);
                    let blob_hash = hasher.finalize();

                    tree_entries.push((
                        file_name.to_string(),
                        "100644".to_string(),
                        blob_hash.to_vec(),
                    ));
                } else if path.is_dir() {
                    // Handle directories - recursively process subdirectory
                    let subdir_hash = write_tree_recursive(&path);
                    tree_entries.push((file_name.to_string(), "40000".to_string(), subdir_hash));
                }
            }

            // Sort entries by name (Git requirement)
            tree_entries.sort_by(|a, b| a.0.cmp(&b.0));

            // Build tree content
            let mut tree_content = Vec::new();
            for (name, mode, hash) in tree_entries {
                tree_content.extend_from_slice(mode.as_bytes());
                tree_content.push(b' ');
                tree_content.extend_from_slice(name.as_bytes());
                tree_content.push(0);
                tree_content.extend_from_slice(&hash);
            }

            let mut tree_object = Vec::new();
            tree_object.extend_from_slice(b"tree ");
            tree_object.extend_from_slice(tree_content.len().to_string().as_bytes());
            tree_object.push(0); // null byte
            tree_object.extend_from_slice(&tree_content);

            // Calculate tree hash
            let mut hasher = Sha1::new();
            hasher.update(&tree_object);
            let tree_hash = hasher.finalize();
            let hash_string = format!("{:x}", tree_hash);

            // Compress and save
            let mut zlib =
                ZlibEncoder::new(std::io::Cursor::new(tree_object), Compression::default());
            let mut compressed_data = Vec::new();
            zlib.read_to_end(&mut compressed_data).unwrap();

            let object_path = format!(".git/objects/{}/{}", &hash_string[0..2], &hash_string[2..]);
            fs::create_dir_all(format!(".git/objects/{}", &hash_string[0..2])).unwrap();
            let mut object_file = File::create(object_path).unwrap();
            object_file.write_all(&compressed_data).unwrap();

            tree_hash.to_vec()
        }

        let root_hash = write_tree_recursive(std::path::Path::new("."));
        let hash_string = root_hash
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>();
        print!("{}", hash_string);
    } else if args[1] == "commit-tree" {
        let tree_hash = args[2].clone();

        // Parse command line arguments for parent (-p) and message (-m)
        let mut parent_hash = String::new();
        let mut message = String::new();

        let mut i = 3;
        while i < args.len() {
            if args[i] == "-p" && i + 1 < args.len() {
                parent_hash = args[i + 1].clone();
                i += 2;
            } else if args[i] == "-m" && i + 1 < args.len() {
                message = args[i + 1].clone();
                i += 2;
            } else {
                i += 1;
            }
        }

        // Build the commit content in Git's format
        let mut commit_content = String::new();
        commit_content.push_str(&format!("tree {}\n", tree_hash));

        if !parent_hash.is_empty() {
            commit_content.push_str(&format!("parent {}\n", parent_hash));
        }

        let author_info = "John Doe <john.doe@example.com> 1719158400 +0000";
        commit_content.push_str(&format!("author {}\n", author_info));
        commit_content.push_str(&format!("committer {}\n", author_info));
        commit_content.push('\n'); // Empty line before message
        commit_content.push_str(&message);
        commit_content.push('\n');

        // Create the full Git object with header
        let content_bytes = commit_content.as_bytes();
        let header = format!("commit {}\0", content_bytes.len());
        let mut commit_object = Vec::new();
        commit_object.extend_from_slice(header.as_bytes());
        commit_object.extend_from_slice(content_bytes);

        // Calculate SHA-1 hash
        let mut hasher = Sha1::new();
        hasher.update(&commit_object);
        let commit_hash_bytes = hasher.finalize();
        let hash_string = format!("{:x}", commit_hash_bytes);

        // Compress and save
        let mut zlib =
            ZlibEncoder::new(std::io::Cursor::new(commit_object), Compression::default());
        let mut compressed_data = Vec::new();
        zlib.read_to_end(&mut compressed_data).unwrap();

        let object_path = format!(".git/objects/{}/{}", &hash_string[0..2], &hash_string[2..]);
        fs::create_dir_all(format!(".git/objects/{}", &hash_string[0..2])).unwrap();
        let mut object_file = File::create(object_path).unwrap();
        object_file.write_all(&compressed_data).unwrap();

        print!("{}", hash_string);
    } else {
        println!("unknown command: {}", args[1])
    }
}
