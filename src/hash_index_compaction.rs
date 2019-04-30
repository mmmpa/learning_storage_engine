use std::fs;
use std::fs::{OpenOptions, ReadDir, DirEntry, File};
use std::io::{Write, BufReader, SeekFrom, BufRead};
use std::collections::HashMap;
use std::io::Seek;
use std::str::FromStr;
use crate::StringError;

struct HashIndexCompaction {
    dir: String,
    rev: String,
    path: String,
    index: HashMap<u32, (String, u64)>
}

impl HashIndexCompaction {
    fn new(dir: &str) -> Self {
        let mut s = Self {
            dir: dir.to_string(),
            rev: "1".to_string(),
            path: String::new(),
            index: HashMap::new(),
        };
        s.prepare();
        s.retrieve();
        s
    }

    fn prepare(&mut self) {
        let _ = fs::create_dir(&self.dir);
        let path = format!("{}/{}", self.dir, self.rev);
        let _ = fs::create_dir(&path);
        self.path = path;
    }

    fn clear(&self) {
        let _ = fs::remove_dir_all(&self.path);
    }

    fn segments(&self) -> Vec<String> {
        fs::read_dir(&self.path).unwrap()
          .map(|o| o.unwrap().path().into_os_string().into_string().unwrap())
          .collect::<Vec<String>>()
    }

    fn last_segment(&self) -> String {
        let mut seg = self.segments();
        seg.sort();
        match seg.last() {
            Some(c) => c.to_string(),
            None => self.create_last_segment(),
        }
    }

    fn retrieve(&mut self) {
        let contents = fs::read_dir(&self.path).unwrap();
        for content in contents {
            // println!("{:?}", content);
        }
    }

    fn retrieve_single(&mut self, path: &str) {
        let file = match fs::File::open(path) {
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
            self.index.insert(id, (path.to_string(), offset));

            offset += len;
            file.seek(SeekFrom::Start(offset)).unwrap();
        }
    }

    fn detect_write_segment(&self) -> Result<(String, File, u64), String> {
        let (path, file, offset) = Self::open(self.last_segment())?;

        if offset < 1000 {
            Ok((path, file, offset))
        } else {
            Ok(Self::open(self.create_last_segment())?)
        }
    }

    fn open(path: String) -> Result<(String, File, u64), String> {
        let file = OpenOptions::new().create(true).append(true).open(&path).str_err("open error")?;
        let offset = file.metadata().unwrap().len();
        Ok((path, file, offset))
    }

    fn create_last_segment(&self) -> String {
        let next = self.segments().len();
        let path = format!("{}/{}.txt", self.path, next + 1);
        fs::write(&path, "").unwrap();
        path
    }

    fn set(&mut self, id: u32, data: &str) -> Result<(), String> {
        let (path, mut file, offset) = self.detect_write_segment()?;

        write!(file, "{}::{}\n", id, data).str_err("write error")?;

        self.index.insert(id, (path, offset));

        Ok(())
    }

    fn get(&self, id: u32) -> Option<String> {
        let (path, offset) = self.index.get(&id)?;

        let mut file = BufReader::new(
            match fs::File::open(path) {
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
    HashIndexCompaction::new("./tmp/segment").clear();

    let mut ms = HashIndexCompaction::new("./tmp/segment");

    ms.set(1, "data 1").unwrap();
    ms.set(2, "data 2-1").unwrap();
    ms.set(2, "data 2-2").unwrap();

    for n in 100..200 {
        ms.set(n, &format!("data {}", n)).unwrap();
    }
    for n in 100..200 {
        ms.set(n, &format!("data {}", n * 10)).unwrap();
    }

    assert_eq!(ms.get(1).unwrap(), "data 1".to_string());
    assert_eq!(ms.get(2).unwrap(), "data 2-2".to_string());
    assert_eq!(ms.get(3), None);
    assert_eq!(ms.get(100).unwrap(), "data 1000".to_string());

    let mut ms = HashIndexCompaction::new("./tmp/segment");

    assert_eq!(ms.get(1).unwrap(), "data 1".to_string());
    assert_eq!(ms.get(2).unwrap(), "data 2-2".to_string());
    assert_eq!(ms.get(3), None);
    assert_eq!(ms.get(100).unwrap(), "data 1000".to_string());
}
