use std::fs;
use std::fs::{OpenOptions, ReadDir, DirEntry, File};
use std::io::{Write, BufReader, SeekFrom, BufRead};
use std::collections::HashMap;
use std::io::Seek;
use std::str::FromStr;
use crate::StringError;
use std::path::{Path, PathBuf};
use std::mem::{swap, replace};

#[derive(Debug)]
struct HashIndexCompaction {
    dir: PathBuf,
    rev: usize,
    path: PathBuf,
    index: HashMap<u32, (PathBuf, u64)>
}

impl HashIndexCompaction {
    fn new(dir: &str, rev: usize) -> Self {
        let mut s = Self {
            dir: PathBuf::from(dir),
            rev,
            path: PathBuf::new(),
            index: HashMap::new(),
        };
        s.prepare();
        s.retrieve();
        s
    }

    fn prepare(&mut self) {
        let _ = fs::create_dir(&self.dir);
        let mut path = self.dir.clone();
        path.push(&self.rev.to_string());
        let _ = fs::create_dir(&path);
        self.path = path;
    }

    fn clear(&self) {
        let _ = fs::remove_dir_all(&self.dir);
    }

    fn segments(&self) -> Vec<PathBuf> {
        fs::read_dir(&self.path).unwrap()
          .map(|o| o.unwrap().path())
          .collect::<Vec<PathBuf>>()
    }

    fn last_segment(&self) -> PathBuf {
        let mut seg = self.segments();
        seg.sort();
        match seg.last() {
            Some(c) => c.clone(),
            None => self.create_last_segment(),
        }
    }

    fn retrieve(&mut self) -> Result<(), String> {
        let mut f = fs::read_dir(&self.path).unwrap()
          .map(|o| o.unwrap().path())
          .collect::<Vec<PathBuf>>();
        f.sort();
        f.iter().for_each(|path| self.retrieve_single(&path));

        Ok(())
    }

    fn retrieve_single(&mut self, path: &PathBuf) {
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
            self.index.insert(id, (path.clone(), offset));

            offset += len;
            file.seek(SeekFrom::Start(offset)).unwrap();
        }
    }

    fn detect_write_segment(&self) -> Result<(PathBuf, File, u64), String> {
        let (path, file, offset) = Self::open(self.last_segment())?;

        if offset < 1000 {
            Ok((path, file, offset))
        } else {
            Ok(Self::open(self.create_last_segment())?)
        }
    }

    fn open(path: PathBuf) -> Result<(PathBuf, File, u64), String> {
        let file = OpenOptions::new().create(true).append(true).open(&path).str_err("open error")?;
        let offset = file.metadata().unwrap().len();
        Ok((path, file, offset))
    }

    fn create_last_segment(&self) -> PathBuf {
        let next = self.segments().len();
        let mut path = self.path.clone();
        path.push(format!("{}.txt", next + 1));
        fs::write(&path, "").unwrap();
        path
    }

    fn compact(&mut self) -> Result<(), String> {
        let mut next = Self::new(self.dir.to_str().unwrap(), self.rev + 1);

        self.index.iter().for_each(|(k, _)| {
            let v = self.get(*k).unwrap();
            next.set(*k, v.as_str()).unwrap();
        });

        swap(self, &mut next);

        Ok(())
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
    HashIndexCompaction::new("./tmp/segment", 1).clear();

    let mut ms = HashIndexCompaction::new("./tmp/segment", 1);

    ms.set(1, "data 1").unwrap();
    ms.set(2, "data 2-1").unwrap();
    ms.set(2, "data 2-2").unwrap();

    for n in 100..200 {
        ms.set(n, &format!("data {}", n)).unwrap();
    }
    for n in 100..200 {
        ms.set(n, &format!("data {}", n)).unwrap();
    }
    for n in 100..200 {
        ms.set(n, &format!("data {}", n * 10)).unwrap();
    }

    assert_eq!(ms.get(100).unwrap(), "data 1000".to_string());
    assert_eq!(ms.get(1).unwrap(), "data 1".to_string());
    assert_eq!(ms.get(2).unwrap(), "data 2-2".to_string());
    assert_eq!(ms.get(3), None);

    let mut ms = HashIndexCompaction::new("./tmp/segment", 1);

    assert_eq!(ms.get(1).unwrap(), "data 1".to_string());
    assert_eq!(ms.get(2).unwrap(), "data 2-2".to_string());
    assert_eq!(ms.get(3), None);
    assert_eq!(ms.get(100).unwrap(), "data 1000".to_string());

    ms.compact().unwrap();

    assert_eq!(ms.rev, 2);

    assert_eq!(ms.get(1).unwrap(), "data 1".to_string());
    assert_eq!(ms.get(2).unwrap(), "data 2-2".to_string());
    assert_eq!(ms.get(3), None);
    assert_eq!(ms.get(100).unwrap(), "data 1000".to_string());
}

