use once_cell::sync::Lazy;
use regex::Regex;

pub static RE_ORDER_ID: Lazy<Regex> = Lazy::new(|| Regex::new(r"#[A-Z0-9]{8}").unwrap());

pub fn extract_phpsessid(set_cookies: &[String]) -> Option<String> {
    for val in set_cookies {
        if let Some(pos) = val.find("PHPSESSID=") {
            let tail = &val[pos + "PHPSESSID=".len()..];
            let sess = tail.split(';').next().unwrap_or("").to_string();
            if !sess.is_empty() {
                return Some(sess);
            }
        }
    }
    None
}

pub fn random_tag() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let alphabet: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    (0..10)
        .map(|_| alphabet[rng.gen_range(0..alphabet.len())] as char)
        .collect()
}
