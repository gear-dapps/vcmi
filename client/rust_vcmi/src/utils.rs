use std::fs::File;
use std::io::BufReader;
use std::io::Read;

pub fn get_file_as_byte_vec(filename: String) -> Vec<u8> {
    println!("Filename: {:?}", filename);
    let f = File::open(&filename).unwrap();
    let mut reader = BufReader::new(f);
    let mut buffer = Vec::new();

    // Read file into vector.
    reader.read_to_end(&mut buffer).unwrap();

    buffer
    // vec![6, 6, 6, 6, 6, 6]
}
