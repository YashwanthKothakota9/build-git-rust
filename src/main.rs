use flate2::read::ZlibDecoder;
use flate2::Compression;
use sha1::{Digest, Sha1};
#[allow(unused_imports)]
use std::env;
#[allow(unused_imports)]
use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

// Helper function to generate object path from hash
fn get_object_path(hash: &str) -> String {
    format!(".git/objects/{}/{}", &hash[0..2], &hash[2..])
}

// Helper function to ensure object directory exists
fn ensure_object_dir(hash: &str) -> std::io::Result<()> {
    fs::create_dir_all(format!(".git/objects/{}", &hash[0..2]))
}

// Helper function to compress data with zlib
fn compress_data(data: &[u8]) -> std::io::Result<Vec<u8>> {
    use flate2::write::ZlibEncoder;
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data)?;
    encoder.finish()
}

// Helper function to decompress data with zlib
fn decompress_data(file_path: &str) -> std::io::Result<Vec<u8>> {
    let file = File::open(file_path)?;
    let mut decompressed_data = Vec::new();
    let mut zlib = ZlibDecoder::new(file);
    zlib.read_to_end(&mut decompressed_data)?;
    Ok(decompressed_data)
}

// Helper function to decompress data from bytes
fn decompress_data_from_bytes(data: &[u8]) -> std::io::Result<Vec<u8>> {
    let mut decoder = ZlibDecoder::new(data);
    let mut result = Vec::new();
    decoder.read_to_end(&mut result)?;
    Ok(result)
}

// Helper function to calculate SHA1 hash
fn calculate_sha1(data: &[u8]) -> String {
    let mut hasher = Sha1::new();
    hasher.update(data);
    let hash = hasher.finalize();
    format!("{:x}", hash)
}

// Helper function to convert hex string to bytes
fn hex_to_bytes(hex_string: &str) -> Vec<u8> {
    let mut bytes = Vec::new();
    for i in (0..hex_string.len()).step_by(2) {
        let byte = u8::from_str_radix(&hex_string[i..i + 2], 16).unwrap();
        bytes.push(byte);
    }
    bytes
}

// Helper function to create git object with header
fn create_git_object(object_type: &str, content: &[u8]) -> Vec<u8> {
    let header = format!("{} {}\0", object_type, content.len());
    let mut object = Vec::new();
    object.extend_from_slice(header.as_bytes());
    object.extend_from_slice(content);
    object
}

// Helper function to write object to git objects directory
fn write_git_object(object_data: &[u8]) -> std::io::Result<String> {
    let hash_string = calculate_sha1(object_data);
    let compressed_data = compress_data(object_data)?;

    ensure_object_dir(&hash_string)?;
    let object_path = get_object_path(&hash_string);
    let mut object_file = File::create(object_path)?;
    object_file.write_all(&compressed_data)?;

    Ok(hash_string)
}

// Helper function to write object to specific parent directory
fn write_object_to_parent(
    parent: &Path,
    object_type: &str,
    content: &[u8],
) -> std::io::Result<String> {
    let object_data = create_git_object(object_type, content);
    let hash = calculate_sha1(&object_data);
    let compressed = compress_data(&object_data)?;

    let obj_dir = parent.join(".git/objects").join(&hash[0..2]);
    fs::create_dir_all(&obj_dir)?;
    fs::write(obj_dir.join(&hash[2..]), compressed)?;

    Ok(hash)
}

// Helper function to read object from parent directory
fn read_object_from_parent(parent: &Path, sha: &str) -> std::io::Result<(String, Vec<u8>)> {
    let obj_path = parent.join(".git/objects").join(&sha[0..2]).join(&sha[2..]);
    let compressed = fs::read(obj_path)?;
    let decompressed = decompress_data_from_bytes(&compressed)?;

    if let Some(null_pos) = decompressed.iter().position(|&b| b == 0) {
        let header = String::from_utf8_lossy(&decompressed[0..null_pos]);
        let parts: Vec<&str> = header.split(' ').collect();
        if parts.len() >= 1 {
            let obj_type = parts[0].to_string();
            let content = decompressed[null_pos + 1..].to_vec();
            return Ok((obj_type, content));
        }
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        "Invalid object format",
    ))
}

// Helper function to read file content
fn read_file_content(file_path: &str) -> std::io::Result<Vec<u8>> {
    fs::read(file_path)
}

// Helper function for hash-object command
fn hash_object_file(file_name: &str) -> std::io::Result<String> {
    let file_data = read_file_content(file_name)?;
    let git_object = create_git_object("blob", &file_data);
    write_git_object(&git_object)
}

// Helper function for cat-file command
fn cat_file_object(object_hash: &str) -> std::io::Result<String> {
    let object_path = get_object_path(object_hash);
    let decompressed_data = decompress_data(&object_path)?;
    let decompressed_string = String::from_utf8(decompressed_data).unwrap();
    let object_parts: Vec<&str> = decompressed_string.split('\0').collect();
    Ok(object_parts[1].to_string())
}

// Helper function for ls-tree command
fn list_tree_names(tree_hash: &str) -> std::io::Result<Vec<String>> {
    let tree_path = get_object_path(tree_hash);
    let decompressed_data = decompress_data(&tree_path)?;

    let null_pos = decompressed_data.iter().position(|&b| b == 0).unwrap();
    let mut entries_data = &decompressed_data[null_pos + 1..];
    let mut names = Vec::new();

    while !entries_data.is_empty() {
        let null_pos = entries_data.iter().position(|&b| b == 0).unwrap();
        let mode_name = String::from_utf8(entries_data[0..null_pos].to_vec()).unwrap();
        let parts: Vec<&str> = mode_name.split(' ').collect();
        let filename = parts[1];
        names.push(filename.to_string());
        entries_data = &entries_data[null_pos + 1 + 20..];
    }

    Ok(names)
}

// Recursive function for write-tree
fn write_tree_recursive(dir_path: &Path) -> std::io::Result<Vec<u8>> {
    let mut tree_entries = Vec::new();

    // Collect all entries (files and directories)
    for entry in fs::read_dir(dir_path)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = path.file_name().unwrap().to_str().unwrap();

        // Skip .git directory
        if file_name == ".git" {
            continue;
        }

        if path.is_file() {
            // Handle files
            let file_data = read_file_content(&path.to_string_lossy())?;
            let git_object = create_git_object("blob", &file_data);
            let blob_hash_string = calculate_sha1(&git_object);
            let hash_bytes = hex_to_bytes(&blob_hash_string);

            tree_entries.push((file_name.to_string(), "100644".to_string(), hash_bytes));
        } else if path.is_dir() {
            // Handle directories - recursively process subdirectory
            let subdir_hash = write_tree_recursive(&path)?;
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

    let tree_object = create_git_object("tree", &tree_content);
    let hash_string = write_git_object(&tree_object)?;
    let hash_bytes = hex_to_bytes(&hash_string);
    Ok(hash_bytes)
}

// Helper function for write-tree command
fn write_tree() -> std::io::Result<String> {
    let root_hash = write_tree_recursive(Path::new("."))?;
    Ok(root_hash
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>())
}

// Helper function for commit-tree command
fn create_commit(
    tree_hash: &str,
    parent_hash: Option<&str>,
    message: &str,
) -> std::io::Result<String> {
    let mut commit_content = String::new();
    commit_content.push_str(&format!("tree {}\n", tree_hash));

    if let Some(parent) = parent_hash {
        commit_content.push_str(&format!("parent {}\n", parent));
    }

    let author_info = "John Doe <john.doe@example.com> 1719158400 +0000";
    commit_content.push_str(&format!("author {}\n", author_info));
    commit_content.push_str(&format!("committer {}\n", author_info));
    commit_content.push('\n'); // Empty line before message
    commit_content.push_str(message);
    commit_content.push('\n');

    let commit_object = create_git_object("commit", commit_content.as_bytes());
    write_git_object(&commit_object)
}

// Initialize repository structure
fn init_repo(parent: &Path) -> std::io::Result<()> {
    fs::create_dir_all(parent.join(".git"))?;
    fs::create_dir_all(parent.join(".git/objects"))?;
    fs::create_dir_all(parent.join(".git/refs"))?;
    fs::create_dir_all(parent.join(".git/refs/heads"))?;
    Ok(())
}

// Parse variable size from delta
fn parse_size(data: &[u8]) -> (usize, &[u8]) {
    let mut size = (data[0] & 0b0111_1111) as usize;
    let mut i = 1;
    let mut offset = 7;

    while data[i - 1] & 0b1000_0000 != 0 {
        size += ((data[i] & 0b0111_1111) as usize) << offset;
        offset += 7;
        i += 1;
    }

    (size, &data[i..])
}

// Apply delta to base content
fn apply_delta(base_content: &[u8], delta_content: &[u8]) -> std::io::Result<Vec<u8>> {
    let mut content = delta_content;

    // Skip base and output sizes
    let (_, remaining) = parse_size(content);
    let (_, remaining) = parse_size(remaining);
    content = remaining;

    let mut target_content = Vec::new();

    while !content.is_empty() {
        let is_copy = content[0] & 0b1000_0000 != 0;

        if is_copy {
            let mut data_ptr = 1;
            let mut offset = 0;
            let mut size = 0;

            // Read offset
            for i in 0..4 {
                if content[0] & (1 << i) != 0 {
                    offset |= (content[data_ptr] as usize) << (i * 8);
                    data_ptr += 1;
                }
            }

            // Read size
            for i in 0..3 {
                if content[0] & (1 << (4 + i)) != 0 {
                    size |= (content[data_ptr] as usize) << (i * 8);
                    data_ptr += 1;
                }
            }

            if size == 0 {
                size = 0x10000;
            }

            content = &content[data_ptr..];
            if offset + size <= base_content.len() {
                target_content.extend_from_slice(&base_content[offset..offset + size]);
            }
        } else {
            let size = content[0] as usize;
            let append = &content[1..size + 1];
            content = &content[size + 1..];
            target_content.extend_from_slice(append);
        }
    }

    Ok(target_content)
}

// Render tree recursively to working directory
fn render_tree(parent: &Path, dir: &Path, sha: &str) -> std::io::Result<()> {
    fs::create_dir_all(dir)?;
    let (_, tree_data) = read_object_from_parent(parent, sha)?;
    let mut tree = tree_data.as_slice();

    while !tree.is_empty() {
        // Parse mode
        let space_pos = tree.iter().position(|&b| b == b' ').unwrap();
        let mode = &tree[..space_pos];
        tree = &tree[space_pos + 1..];

        // Parse name
        let null_pos = tree.iter().position(|&b| b == 0).unwrap();
        let name = String::from_utf8_lossy(&tree[..null_pos]);
        tree = &tree[null_pos + 1..];

        // Parse SHA
        let sha = hex::encode(&tree[..20]);
        tree = &tree[20..];

        match mode {
            b"40000" => {
                // Directory
                render_tree(parent, &dir.join(name.as_ref()), &sha)?;
            }
            b"100644" => {
                // Regular file
                let (_, content) = read_object_from_parent(parent, &sha)?;
                fs::write(dir.join(name.as_ref()), content)?;
            }
            _ => {
                // Skip other modes
            }
        }
    }

    Ok(())
}

// Helper function to parse default branch from refs response
fn parse_default_branch(refs: &str) -> Option<String> {
    for line in refs.lines() {
        if line.contains("symref=HEAD:") {
            if let Some(start) = line.find("symref=HEAD:") {
                let symref_part = &line[start + 12..]; // Skip "symref=HEAD:"
                if let Some(end) = symref_part.find(' ') {
                    let branch_ref = &symref_part[..end];
                    if let Some(branch_name) = branch_ref.strip_prefix("refs/heads/") {
                        return Some(branch_name.to_string());
                    }
                } else {
                    // If there's no space, take the rest of the line
                    if let Some(branch_name) = symref_part.strip_prefix("refs/heads/") {
                        return Some(branch_name.to_string());
                    }
                }
            }
        }
    }
    None
}

// Helper function to get head commit from refs response
fn get_head_commit(refs: &str) -> Option<String> {
    // Look for refs/heads/master or refs/heads/main line
    for line in refs.lines() {
        if line.ends_with("refs/heads/master") || line.ends_with("refs/heads/main") {
            // Line format: "003f47b37f1a82bfe85f6d8df52b6258b75e4343b7fd refs/heads/master"
            // Skip the length prefix (4 hex chars) and extract the SHA
            if line.len() >= 48 {
                // 4 (length) + 40 (SHA) + 4 (space + "refs")
                let sha_part = &line[4..44]; // Extract exactly 40 characters after length prefix
                if sha_part.len() == 40 && sha_part.chars().all(|c| c.is_ascii_hexdigit()) {
                    return Some(sha_part.to_string());
                }
            }
        }
    }

    None
}

// Helper function to process ref delta objects
fn process_ref_delta(
    parent: &Path,
    base_sha_bytes: &[u8],
    delta_data: &[u8],
) -> std::io::Result<()> {
    let base_sha = hex::encode(base_sha_bytes);

    if let Ok((base_type, base_content)) = read_object_from_parent(parent, &base_sha) {
        if let Ok(target_content) = apply_delta(&base_content, delta_data) {
            write_object_to_parent(parent, &base_type, &target_content)?;
        }
    }

    Ok(())
}

// Main clone function using improved logic
fn clone_repository(repository_url: &str, local_path: &str) -> std::io::Result<()> {
    let parent = Path::new(local_path);

    // Create target directory structure
    if let Some(parent_dir) = parent.parent() {
        fs::create_dir_all(parent_dir)?;
    }
    fs::create_dir_all(parent)?;

    init_repo(parent)?;

    let client = reqwest::blocking::Client::new();

    // Fetch refs with improved error handling
    let smart_url = format!("{}/info/refs?service=git-upload-pack", repository_url);
    // println!("Requesting refs from: {}", smart_url);

    let refs_response = client
        .get(&smart_url)
        .send()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    if !refs_response.status().is_success() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to get refs: {}", refs_response.status()),
        ));
    }

    let refs_bytes = refs_response
        .bytes()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    let refs_data = String::from_utf8_lossy(&refs_bytes);
    // eprintln!("Refs data: {}", refs_data);

    let head_commit = get_head_commit(&refs_data)
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "No head commit found!"))?;

    // Parse the default branch from symbolic ref
    let default_branch = parse_default_branch(&refs_data).unwrap_or_else(|| "main".to_string());

    // Create pack request
    let pack_url = format!("{}/git-upload-pack", repository_url);
    let pack_request = format!(
        "0032want {}\n\
         0000\
         0009done\n",
        head_commit
    );

    // eprintln!("Requesting packfile from: {}", pack_url);
    let pack_response = client
        .post(&pack_url)
        .header("Content-Type", "application/x-git-upload-pack-request")
        .header("Accept", "application/x-git-upload-pack-result")
        .body(pack_request)
        .send()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    if !pack_response.status().is_success() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to get packfile: {}", pack_response.status()),
        ));
    }

    let pack_data = pack_response
        .bytes()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    // eprintln!("Received packfile of size: {} bytes", pack_data.len());

    if pack_data.is_empty() {
        eprintln!("Received empty response from server");
        eprintln!("Creating a minimal repository");

        // Write HEAD and refs using correct branch
        fs::write(
            parent.join(".git/HEAD"),
            format!("ref: refs/heads/{}\n", default_branch),
        )?;
        let branch_ref = parent.join(".git/refs/heads").join(&default_branch);
        if let Some(parent_dir) = branch_ref.parent() {
            fs::create_dir_all(parent_dir)?;
        }
        fs::write(branch_ref, format!("{}\n", head_commit))?;

        return Ok(());
    }

    // Find packfile start
    let mut pack_start = 0;
    for (i, chunk) in pack_data.windows(4).enumerate() {
        if chunk == b"PACK" {
            pack_start = i;
            break;
        }
    }

    if pack_start == 0 && &pack_data[0..4] != b"PACK" {
        eprintln!("Response: {}", String::from_utf8_lossy(&pack_data));
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Could not find packfile in response",
        ));
    }

    eprintln!("Packfile starts at offset: {}", pack_start);

    // Process packfile with improved logic
    let pack_file_data = &pack_data[pack_start..];
    if pack_file_data.len() < 12 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Packfile too short",
        ));
    }

    // Check header
    if &pack_file_data[0..4] != b"PACK" {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Invalid packfile header",
        ));
    }

    // Check version
    let version = u32::from_be_bytes([
        pack_file_data[4],
        pack_file_data[5],
        pack_file_data[6],
        pack_file_data[7],
    ]);
    if version != 2 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Unsupported packfile version: {}", version),
        ));
    }

    // Get number of objects
    let num_objects = u32::from_be_bytes([
        pack_file_data[8],
        pack_file_data[9],
        pack_file_data[10],
        pack_file_data[11],
    ]);
    println!("Processing packfile with {} objects", num_objects);

    let mut data = &pack_file_data[12..];
    let mut processed = 0;

    while processed < num_objects && !data.is_empty() && data.len() > 20 {
        println!("Processing object {}/{}", processed + 1, num_objects);

        // Read object header
        let first_byte = data[0];
        let obj_type = (first_byte & 0b0111_0000) >> 4;
        let mut size = (first_byte & 0b0000_1111) as u64;
        let mut i = 1;
        let mut shift = 4;

        // Read variable length size
        while data[i - 1] & 0b1000_0000 != 0 && i < data.len() {
            size |= ((data[i] & 0b0111_1111) as u64) << shift;
            shift += 7;
            i += 1;
        }

        data = &data[i..];

        println!("Object type: {}, size: {}", obj_type, size);

        match obj_type {
            1..=4 => {
                // Regular objects (commit, tree, blob, tag)
                let mut decoder = ZlibDecoder::new(data);
                let mut decompressed = Vec::new();

                if decoder.read_to_end(&mut decompressed).is_ok() {
                    let consumed = decoder.total_in() as usize;
                    data = &data[consumed..];

                    let content = &decompressed[..size.min(decompressed.len() as u64) as usize];
                    let type_name = match obj_type {
                        1 => "commit",
                        2 => "tree",
                        3 => "blob",
                        4 => "tag",
                        _ => "unknown",
                    };

                    if let Err(e) = write_object_to_parent(parent, type_name, content) {
                        eprintln!("Error writing object: {}", e);
                    }
                } else {
                    eprintln!("Failed to decompress object {}", processed + 1);
                    break;
                }
            }
            7 => {
                // ref_delta
                if data.len() < 20 {
                    eprintln!("Not enough data for ref delta");
                    break;
                }

                let base_sha_bytes = &data[..20];
                data = &data[20..];

                let mut decoder = ZlibDecoder::new(data);
                let mut delta_content = Vec::new();

                if decoder.read_to_end(&mut delta_content).is_ok() {
                    let consumed = decoder.total_in() as usize;
                    data = &data[consumed..];

                    if let Err(e) = process_ref_delta(parent, base_sha_bytes, &delta_content) {
                        eprintln!("Error processing ref delta: {}", e);
                    }
                } else {
                    eprintln!("Failed to decompress ref delta {}", processed + 1);
                    break;
                }
            }
            6 => {
                // ofs_delta - skip for now
                eprintln!("Skipping ofs_delta object");
                let mut decoder = ZlibDecoder::new(data);
                let mut temp = Vec::new();
                if decoder.read_to_end(&mut temp).is_ok() {
                    let consumed = decoder.total_in() as usize;
                    data = &data[consumed..];
                }
            }
            _ => {
                eprintln!("Unknown object type: {}", obj_type);
                break;
            }
        }

        processed += 1;

        if processed >= num_objects {
            break;
        }
    }

    // Write HEAD and refs using correct branch
    fs::write(
        parent.join(".git/HEAD"),
        format!("ref: refs/heads/{}\n", default_branch),
    )?;
    let branch_ref = parent.join(".git/refs/heads").join(&default_branch);
    if let Some(parent_dir) = branch_ref.parent() {
        fs::create_dir_all(parent_dir)?;
    }
    fs::write(branch_ref, format!("{}\n", head_commit))?;

    // Create working directory from HEAD commit
    if let Err(e) = create_working_directory_from_commit(parent, &head_commit) {
        eprintln!("Warning: Failed to create working directory: {}", e);
    }

    Ok(())
}

// Helper function to create working directory from commit
fn create_working_directory_from_commit(parent: &Path, head_commit: &str) -> std::io::Result<()> {
    println!("Creating working directory from commit {}", head_commit);

    // Read the commit object
    let (_, commit_data) = read_object_from_parent(parent, head_commit)?;
    let commit_str = String::from_utf8_lossy(&commit_data);
    println!("Commit data: {}", commit_str);

    // Extract tree SHA from commit
    let tree_sha = commit_str
        .lines()
        .find(|line| line.starts_with("tree "))
        .and_then(|line| line.split_whitespace().nth(1))
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Could not find tree SHA in commit",
            )
        })?;

    println!("Root tree SHA: {}", tree_sha);

    // Render the tree to working directory
    render_tree(parent, parent, tree_sha)?;

    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().collect();

    match args[1].as_str() {
        "init" => {
            fs::create_dir(".git").unwrap();
            fs::create_dir(".git/objects").unwrap();
            fs::create_dir(".git/refs").unwrap();
            fs::write(".git/HEAD", "ref: refs/heads/main\n").unwrap();
            println!("Initialized git directory");
        }

        "cat-file" => {
            let object_hash = &args[3];
            match cat_file_object(object_hash) {
                Ok(content) => print!("{}", content),
                Err(e) => eprintln!("Error: {}", e),
            }
        }

        "hash-object" => {
            let file_name = &args[3];
            match hash_object_file(file_name) {
                Ok(hash) => print!("{}", hash),
                Err(e) => eprintln!("Error: {}", e),
            }
        }

        "ls-tree" if args[2] == "--name-only" => {
            let tree_hash = &args[3];
            match list_tree_names(tree_hash) {
                Ok(names) => {
                    for name in names {
                        println!("{}", name);
                    }
                }
                Err(e) => eprintln!("Error: {}", e),
            }
        }

        "write-tree" => match write_tree() {
            Ok(hash) => print!("{}", hash),
            Err(e) => eprintln!("Error: {}", e),
        },

        "commit-tree" => {
            let tree_hash = &args[2];
            let mut parent_hash = None;
            let mut message = String::new();

            let mut i = 3;
            while i < args.len() {
                match args[i].as_str() {
                    "-p" if i + 1 < args.len() => {
                        parent_hash = Some(args[i + 1].as_str());
                        i += 2;
                    }
                    "-m" if i + 1 < args.len() => {
                        message = args[i + 1].clone();
                        i += 2;
                    }
                    _ => i += 1,
                }
            }

            match create_commit(tree_hash, parent_hash, &message) {
                Ok(hash) => print!("{}", hash),
                Err(e) => eprintln!("Error: {}", e),
            }
        }

        "clone" => {
            let repository_url = &args[2];
            let local_path = &args[3];

            match clone_repository(repository_url, local_path) {
                Ok(_) => println!("Repository cloned successfully"),
                Err(e) => eprintln!("Error cloning repository: {}", e),
            }
        }

        _ => println!("unknown command: {}", args[1]),
    }
}
