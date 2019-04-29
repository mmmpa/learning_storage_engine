use std::fs;
use std::fs::OpenOptions;
use std::io::{Write, BufReader, BufRead};
use std::str::FromStr;

struct MostSimple {
    path: String,
}

impl MostSimple {
    fn new(path: &str) -> MostSimple {
        let _ = fs::remove_file(path);

        MostSimple {
            path: path.to_string(),
        }
    }

    fn set(&self, id: u32, data: &str) -> Result<(), String> {
        let mut file = match OpenOptions::new().create(true).append(true).open(&self.path) {
            Err(_) => return Err("open error".to_string()),
            Ok(f) => f,
        };
        match write!(file, "{}::{}\n", id, data) {
            Err(_) => return Err("write error".to_string()),
            _ => (),
        };

        Ok(())
    }

    fn get(&self, id: u32) -> Option<String> {
        let file = BufReader::new(
            match fs::File::open(&self.path) {
                Err(_) => return None,
                Ok(f) => f,
            }
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
fn test_get_set() {
    let ms = MostSimple::new("./tmp/most_simple.txt");

    ms.set(1, "data 1").unwrap();
    ms.set(2, "data 2-1").unwrap();
    ms.set(2, "data 2-2").unwrap();
    assert_eq!(ms.get(1).unwrap(), "data 1".to_string());
    assert_eq!(ms.get(2).unwrap(), "data 2-2".to_string());
    assert_eq!(ms.get(3), None);
}
