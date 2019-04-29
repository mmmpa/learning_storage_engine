use std::fs;
use std::fs::OpenOptions;
use std::io::{Write, BufReader, SeekFrom, BufRead};
use std::collections::HashMap;
use std::io::Seek;
use std::str::FromStr;

struct HashIndex {
    path: String,
    index: HashMap<u32, u64>
}

impl HashIndex {
    fn new(path: &str) -> Self {
        let mut s = Self {
            path: path.to_string(),
            index: HashMap::new(),
        };
        s.retrieve();
        s
    }

    fn clear(&self) {
        let _ = fs::remove_file(&self.path);
    }

    fn retrieve(&mut self) {
        let file = match fs::File::open(&self.path) {
            Err(_) => return (),
            Ok(f) => f,
        };
        let eof = file.metadata().unwrap().len();
        let mut file = BufReader::new(file);

        let _ = file.seek(SeekFrom::Start(0));
        let mut offset = 0;

        while offset < eof {
            let mut buf = String::new();
            let len = file.read_line(&mut buf).unwrap() as u64;

            let s: Vec<&str> = buf.split("::").collect();
            let id = u32::from_str(s.get(0).unwrap()).unwrap();
            self.index.insert(id, offset);

            offset += len;
            file.seek(SeekFrom::Start(offset)).unwrap();
        }
    }

    fn set(&mut self, id: u32, data: &str) -> Result<(), String> {
        let mut file = match OpenOptions::new().create(true).append(true).open(&self.path) {
            Err(_) => return Err("open error".to_string()),
            Ok(f) => f,
        };
        let offset = file.metadata().unwrap().len();

        match write!(file, "{}::{}\n", id, data) {
            Err(_) => return Err("write error".to_string()),
            _ => (),
        };

        self.index.insert(id, offset);

        Ok(())
    }

    fn get(&self, id: u32) -> Option<String> {
        let offset = self.index.get(&id)?;

        let mut file = BufReader::new(
            match fs::File::open(&self.path) {
                Err(_) => return None,
                Ok(f) => f,
            }
        );

        file.seek(SeekFrom::Start(*offset)).unwrap();
        let mut line = String::new();
        file.read_line(&mut line).unwrap();

        let s: Vec<&str> = line.split("::").collect();
        let data = s.get(1).unwrap();

        Some(data[..data.len() - 1].to_string())
    }
}

#[test]
fn test_get_set() {
    let mut ms = HashIndex::new("./tmp/hash_index.txt");

    ms.set(1, "data 1").unwrap();
    ms.set(2, "data 2-1").unwrap();
    ms.set(2, "data 2-2").unwrap();
    assert_eq!(ms.get(1).unwrap(), "data 1".to_string());
    assert_eq!(ms.get(2).unwrap(), "data 2-2".to_string());
    assert_eq!(ms.get(3), None);

    let ms = HashIndex::new("./tmp/hash_index.txt");

    assert_eq!(ms.get(1).unwrap(), "data 1".to_string());
    assert_eq!(ms.get(2).unwrap(), "data 2-2".to_string());
    assert_eq!(ms.get(3), None);

    ms.clear();

    let ms = HashIndex::new("./tmp/hash_index.txt");

    assert_eq!(ms.get(1), None);
    assert_eq!(ms.get(2), None);
    assert_eq!(ms.get(3), None);
}
