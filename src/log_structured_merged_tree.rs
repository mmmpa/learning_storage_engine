use std::collections::btree_map::{BTreeMap};
use std::path::PathBuf;
use std::fs;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader};
use std::str::FromStr;
use intrusive_collections::rbtree::RBTree;
use intrusive_collections::{RBTreeLink, KeyAdapter};
use rand::thread_rng;
use rand::seq::SliceRandom;
use std::ops::{RangeBounds, Range};

struct LSM {
    c0: BTreeMap<u32, String>,
    c1: Vec<u32>,
    c0_threshold: usize,
    c1_threshold: usize,
    rev: usize,
}

impl LSM {
    fn new() -> Self {
        Self {
            c0: BTreeMap::new(),
            c1: vec![],
            c0_threshold: 10,
            c1_threshold: 256,
            rev: 0,
        }
    }

    fn clear() {
        let _ = fs::remove_dir_all("./tmp/lsm");
    }

    fn set(&mut self, id: u32, data: &str) -> Result<(), String> {
        self.c0.insert(id, data.to_string());

        if self.c0_threshold <= self.c0.len() {
            self.merge();
        }

        Ok(())
    }

    fn get(&self, id: u32) -> Option<String> {
        match self.c0.get(&id) {
            Some(s) => Some(s.to_string()),
            _ => self.search(id),
        }
    }

    fn dir(&self) -> PathBuf {
        let mut path = PathBuf::from("./tmp/lsm/");
        let _ = fs::create_dir(&path);
        path.push(self.rev.to_string());
        let _ = fs::create_dir(&path);
        path
    }

    fn sorted_segment_path(&self) -> Vec<PathBuf> {
        self.c1.iter().map(|id| self.create_path(id.clone())).collect::<Vec<PathBuf>>()
    }

    fn merge(&mut self) {
        let mut c1 = self.sorted_segment_path();

        self.rev += 1;
        self.c1.clear();

        let mut c0 = self.c0.iter();
        let mut c1 = c1.iter()
          .map(|path| BufReader::new(OpenOptions::new().read(true).open(path).unwrap()))
          .flat_map(|buf| buf.lines())
          .map(|line| Self::parse(&line.unwrap()));

        let mut c0_now = c0.next();
        let mut c1_now = c1.next();

        let mut stack = String::new();
        let mut last_id = 0;

        loop {
            match (&c0_now, &c1_now) {
                (Some(new), Some(old)) => {
                    let new_id = new.0.clone();
                    let old_id = old.0.clone();

                    if new_id < old_id {
                        Self::add_k_v(&mut stack, (&new.0, &new.1));
                        last_id = new_id;
                        c0_now = c0.next();
                    } else if new_id > old_id {
                        Self::add_k_v(&mut stack, (&old.0, &old.1));
                        last_id = old_id;
                        c1_now = c1.next();
                    } else {
                        c1_now = c1.next();
                    }
                },
                (Some(new), None) => {
                    Self::add_k_v(&mut stack, (&new.0, &new.1));
                    last_id = new.0.clone();
                    c0_now = c0.next();
                },
                (None, Some(old)) => {
                    Self::add_k_v(&mut stack, (&old.0, &old.1));
                    last_id = old.0;
                    c1_now = c1.next();
                },
                (None, None) => {
                    break;
                }
            }

            if stack.len() > self.c1_threshold {
                self.write_segment(last_id, &stack);
                self.c1.push(last_id);
                stack = String::new();
            }
        }

        if stack.len() != 0 {
            self.write_segment(last_id, &stack);
            self.c1.push(last_id);
        }

        self.c0.clear();
    }

    fn write_segment(&self, last_id: u32, data: &str) {
        fs::write(self.create_path(last_id), data);
    }

    fn create_path(&self, id: u32) -> PathBuf {
        let mut path = self.dir();
        path.push(id.to_string());
        path
    }

    fn parse(line: &str) -> (u32, String) {
        let s: Vec<&str> = line.split("::").collect();
        let id = u32::from_str(s.get(0).unwrap()).unwrap();
        let data = s.get(1).unwrap();
        (id, data.to_string())
    }

    fn add_k_v(s: &mut String, kv: (&u32, &String)) {
        s.push_str(&kv.0.to_string());
        s.push_str("::");
        s.push_str(kv.1);
        s.push('\n');
    }

    fn create_first_segment(&mut self) {
        let mut m = String::with_capacity(self.c0_threshold * 100);
        let (first_key, _) = self.c0.iter().next().unwrap();
        self.c0.iter().for_each(|kv| Self::add_k_v(&mut m, kv));
        let mut path = self.dir();
        path.push(first_key.to_string());
        fs::write(path, m);
        self.c0.clear();
    }

    fn search(&self, id: u32) -> Option<String> {
        let nearest = self.c1.iter().find(|n| n >= &&id)?;

        let file = BufReader::new(
            fs::File::open(self.create_path(nearest.clone())).unwrap()
        );

        let mut matched = None;

        for line in file.lines() {
            let line = line.unwrap();
            let s: Vec<&str> = line.split("::").collect();
            let id_now = u32::from_str(s.get(0).unwrap()).unwrap();
            let data = s.get(1).unwrap();
            if id == id_now { matched = Some(data.to_string()); }
        }

        matched
    }
}

#[test]
fn test_map() {
    let mut lsm = LSM::new();
    lsm.set(2, "data 2").unwrap();
    lsm.set(5, "data 5").unwrap();
    lsm.set(0, "data 0").unwrap();
    lsm.set(2, "data 2").unwrap();

    assert_eq!(lsm.c0.len(), 3);
    assert_eq!(lsm.c0.keys().map(|n| n.clone()).collect::<Vec<u32>>(), vec![0, 2, 5]);
}

#[test]
fn test_get_set() {
    let mut lsm = LSM::new();
    lsm.set(2, "data 2").unwrap();
    lsm.set(5, "data 5").unwrap();
    lsm.set(0, "data 0").unwrap();
    lsm.set(2, "data 2-2").unwrap();

    assert_eq!(lsm.get(0).unwrap(), "data 0");
    assert_eq!(lsm.get(2).unwrap(), "data 2-2");
    assert_eq!(lsm.get(5).unwrap(), "data 5");
    assert_eq!(lsm.get(14), None);
}

#[test]
fn test_compact() {
    LSM::clear();
    let mut lsm = LSM::new();

    for n in r(1..20) {
        lsm.set(n, &format!("data record {}", n)).unwrap();
    }

    for n in r(5..10) {
        lsm.set(n, &format!("data record 2 {}", n)).unwrap();
    }

    for n in r(0..100) {
        lsm.set(n * 2, &format!("data record 3 {}", n * 2)).unwrap();
    }

    assert_eq!(lsm.get(5).unwrap(), "data record 2 5");
    assert_eq!(lsm.get(160).unwrap(), "data record 3 160");
    assert_eq!(lsm.get(198).unwrap(), "data record 3 198");
}

fn r(range: Range<u32>) -> Vec<u32> {
    let mut vec: Vec<u32> = range.collect();
    vec.shuffle(&mut thread_rng());
    vec
}
