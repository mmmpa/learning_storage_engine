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
        let c1 = fs::read_dir(self.dir()).unwrap().map(|e| e.unwrap().path()).collect::<Vec<PathBuf>>();

        self.rev += 1;

        if c1.len() == 0 {
            return self.create_first_segment();
        }

        let mut c0_iter = self.c0.iter();
        let mut c1_iter = c1.iter();
        let mut kv_sd = c0_iter.next();
        let mut written = false;

        for old_path in c1_iter {
            let mut m = String::new();
            let mut last_id = 0;

            // already completed tracing Hash
            if let None = kv_sd {
                let mut path = self.dir();
                path.push(old_path.file_name().unwrap());
                fs::copy(old_path, path);
                continue;
            }

            let segment = BufReader::new(OpenOptions::new().read(true).open(old_path).unwrap());

            let mut line_iter = segment.lines();
            let mut line_s = line_iter.next();
            while let Some(line) = line_s {
                let mut old_kv = Self::parse(&line.unwrap());

                while let Some(kv) = kv_sd {
                    if kv.0 <= &old_kv.0 {
                        Self::add_k_v(&mut m, kv);
                        last_id = kv.0.clone();
                        kv_sd = c0_iter.next();

                        while kv.0 == &old_kv.0 {
                            line_s = line_iter.next();
                            if let Some(line) = &line_s {
                                old_kv = Self::parse(&line.unwrap());
                            }
                        }
                    } else {
                        break;
                    }
                }

                Self::add_k_v(&mut m, (&old_kv.0, &old_kv.1));
                last_id = old_kv.0.clone();
            }

            let mut path = self.dir();
            path.push(last_id.to_string());
            fs::write(path, m);
        }

        // rests in Hash
        {
            let mut m = String::new();
            let mut last_id = 0;

            if let Some(kv) = kv_sd {
                Self::add_k_v(&mut m, kv);
                last_id = kv.0.clone();
            }

            for kv in c0_iter {
                Self::add_k_v(&mut m, kv);
                last_id = kv.0.clone();
            }

            let mut path = self.dir();
            path.push(last_id.to_string());
            fs::write(path, m);
        }

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

    for n in 1..20 {
        lsm.set(n, &format!("data {}", n)).unwrap();
    }
}
