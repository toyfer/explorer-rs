//! Windows CI performance bench. Writes bench-summary.json when EXPLORER_RS_BENCH_JSON=1.

use std::cmp::Ordering;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

#[derive(Clone)]
struct FileEntry {
    name: String,
    is_dir: bool,
}

fn list_blocking(dir: &Path) -> Vec<FileEntry> {
    let mut entries = Vec::new();
    let Ok(rd) = fs::read_dir(dir) else {
        return entries;
    };
    for e in rd.flatten() {
        let path = e.path();
        let Ok(md) = fs::metadata(&path) else {
            continue;
        };
        let name = path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        entries.push(FileEntry {
            name,
            is_dir: md.is_dir(),
        });
    }
    entries
}

fn natural_cmp(a: &str, b: &str) -> Ordering {
    let mut ac = a.chars().peekable();
    let mut bc = b.chars().peekable();
    loop {
        match (ac.peek().copied(), bc.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(ca), Some(cb)) => {
                if ca.is_ascii_digit() && cb.is_ascii_digit() {
                    let mut na = 0u64;
                    while let Some(c) = ac.peek().copied() {
                        if c.is_ascii_digit() {
                            ac.next();
                            na = na.saturating_mul(10).saturating_add((c as u8 - b'0') as u64);
                        } else {
                            break;
                        }
                    }
                    let mut nb = 0u64;
                    while let Some(c) = bc.peek().copied() {
                        if c.is_ascii_digit() {
                            bc.next();
                            nb = nb.saturating_mul(10).saturating_add((c as u8 - b'0') as u64);
                        } else {
                            break;
                        }
                    }
                    match na.cmp(&nb) {
                        Ordering::Equal => continue,
                        o => return o,
                    }
                } else {
                    let ca = ac.next().unwrap().to_ascii_lowercase();
                    let cb = bc.next().unwrap().to_ascii_lowercase();
                    match ca.cmp(&cb) {
                        Ordering::Equal => continue,
                        o => return o,
                    }
                }
            }
        }
    }
}

fn sort_name(entries: &mut [FileEntry]) {
    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| natural_cmp(&a.name, &b.name))
    });
}

fn tmp_dir() -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "explorer-rs-bench-5k-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&p).unwrap();
    p
}

#[test]
fn bench_list_5k() {
    const N: usize = 5_000;
    let dir = tmp_dir();
    for i in 0..N {
        fs::write(dir.join(format!("f{i}.txt")), b"x").unwrap();
    }

    let mut samples_ms = Vec::new();
    for _ in 0..5 {
        let t0 = Instant::now();
        let mut entries = list_blocking(&dir);
        sort_name(&mut entries);
        let ms = t0.elapsed().as_secs_f64() * 1000.0;
        assert_eq!(entries.len(), N);
        samples_ms.push(ms);
    }
    samples_ms.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let p50 = samples_ms[samples_ms.len() / 2];
    let p90 = samples_ms[(samples_ms.len() * 9) / 10];

    eprintln!("bench_list_5k p50_ms={p50:.2} p90_ms={p90:.2} samples={samples_ms:?}");

    assert!(
        p50 < 15_000.0,
        "list 5k catastrophically slow: p50={p50}ms"
    );

    if std::env::var("EXPLORER_RS_BENCH_JSON").ok().as_deref() == Some("1") {
        let soft_ok = p50 < 500.0;
        let json = format!(
            "{{\"bench\":\"list_5k\",\"n\":{N},\"p50_ms\":{p50:.3},\"p90_ms\":{p90:.3},\"samples_ms\":{samples:?},\"target_p50_ms\":500,\"os\":\"{os}\",\"ok_soft\":{soft_ok}}}",
            samples = samples_ms,
            os = std::env::consts::OS,
        );
        fs::write("bench-summary.json", json).expect("write bench-summary.json");
    }

    let _ = fs::remove_dir_all(dir);
}
