use rand::{self, Rng};

pub fn gen_rand_str(len: usize) -> String {
    let mut ret = String::new();
    let mut rng = rand::thread_rng();
    for _ in 0..len {
        let next_char: char = rng.gen_range('a'..='z');
        ret.push(next_char);
    }
    ret
}