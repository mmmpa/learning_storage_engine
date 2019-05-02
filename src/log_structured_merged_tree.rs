use std::collections::btree_map::BTreeMap;
use std::path::PathBuf;
use std::fs;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader};
use std::str::FromStr;

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
            c1_threshold: 20,
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

    fn merge(&mut self) {
        let mut c1 = fs::read_dir(self.dir()).unwrap()
          .map(|e| e.unwrap().path())
          .collect::<Vec<PathBuf>>();
        c1.sort();

        let mut c1 = c1.iter()
          .map(|path| BufReader::new(OpenOptions::new().read(true).open(path).unwrap()))
          .flat_map(|buf| buf.lines())
          .map(|line| Self::parse(&line.unwrap()));

        let mut c0 = self.c0.iter();

        let mut c0_now = c0.next();
        let mut c1_now = c1.next();

        let mut stack = String::new();
        loop {
            match (&c0_now, &c1_now) {
                (Some(new), Some(old)) => {
                    let new_id = new.0.clone();
                    let old_id = old.0.clone();

                    if new_id < old_id {
                        Self::add_k_v(&mut stack, (&new.0, &new.1));
                        c0_now = c0.next();
                    } else if new_id > old_id {
                        Self::add_k_v(&mut stack, (&old.0, &old.1));
                        c1_now = c1.next();
                    } else {
                        c1_now = c1.next();
                    }
                },
                (None, Some(old)) => {
                    Self::add_k_v(&mut stack, (&old.0, &old.1));
                    c1_now = c1.next();
                },
                (Some(new), None) => {
                    Self::add_k_v(&mut stack, (&new.0, &new.1));
                    c0_now = c0.next();
                },
                (None, None) => {
                    break;
                }
            }
        }

        println!("stack {}", stack);
        self.rev += 1;
        let mut path = self.dir();
        path.push("0".to_string());
        fs::write(path, stack);

        self.c0.clear();
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
        None
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

    for n in 1..100 {
        lsm.set(n, &format!("data {}", n)).unwrap();
    }

    for n in 5..20 {
        lsm.set(n, &format!("data 2 {}", n)).unwrap();
    }
}
