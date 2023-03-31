use blake2::{digest::generic_array::sequence::Lengthen, Blake2b, Digest};
use rand::Rng;
use std::thread;
use std::time::Instant;
use std::{env, fs};

pub type Blake2b256 = Blake2b<blake2::digest::consts::U32>;

fn mk_nonce_digits(num_nonce_digits: u8) -> String {
    let mut nonce = String::from("");
    for _ in 0..num_nonce_digits {
        let random_digit = rand::thread_rng().gen_range(0..10).to_string();
        nonce.push(random_digit.chars().last().expect("no digit?"));
    }
    nonce
}

const NUM_NONCE_DIGITS: u8 = 16;

fn mk_nonce() -> String {
    let nonce_digits = mk_nonce_digits(NUM_NONCE_DIGITS);
    format!("(* Vanity nonce: {nonce_digits} *)\n")
}

fn get_hash(vanity_nonce: &str, mut proto_hash: Blake2b256) -> String {
    proto_hash.update(vanity_nonce.as_bytes());
    let raw_hash = proto_hash.finalize().prepend(0xaa).prepend(0x02);
    bs58::encode(raw_hash).with_check().into_string()
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let file_path = &args[1];
    let vanity = &args[2];

    println!("Looking for vanity hash {vanity} for {file_path}");

    let data_as_bytes = fs::read(file_path).expect("Unable to read file");
    let data = String::from_utf8(data_as_bytes.clone()).expect("Unable to decode as UTF-8 text");
    let vanity_comment_start = "(* Vanity nonce:";
    let maybe_vanity_start = data.rfind(vanity_comment_start);

    match maybe_vanity_start {
        Some(vanity_start) => {
            //First 4 bytes are truncated (not used in proto hashing)
            let up_to_vanity = &data_as_bytes[4..vanity_start];
            //dbg!(str::from_utf8(up_to_vanity).unwrap());
            let mut hasher = Blake2b256::new();
            hasher.update(up_to_vanity);
            let mut count = 0u64;
            let mut vanity_count = 0u64;
            let t0 = Instant::now();
            let mut start_time = Instant::now();
            let mut total_nonce_time = 0;
            let mut total_hash_time = 0;
            loop {
                let nonce_time = Instant::now();
                let nonce = mk_nonce();
                total_nonce_time += nonce_time.elapsed().as_nanos();
                let hash_time = Instant::now();
                let hash = get_hash(&nonce, hasher.clone());
                total_hash_time += hash_time.elapsed().as_nanos();
                count += 1;
                if hash.starts_with(vanity) {
                    vanity_count += 1;
                    let elapsed = start_time.elapsed().as_secs();
                    let elapsed_total = t0.elapsed().as_secs();
                    let total_rate = 60 * vanity_count / elapsed_total;
                    print!("hash: {hash}\n{nonce}");
                    println!("elapsed: {elapsed}s\nattempt: {count}\ntotal: {vanity_count} ({total_rate} per minute)");
                    println!("n time: {total_nonce_time}");
                    println!("h time: {total_hash_time}");

                    start_time = Instant::now();
                    total_hash_time = 0;
                    total_nonce_time = 0;
                }
            }
        }
        None => println!("Vanity comment line start '{vanity_comment_start}' is not found "),
    }
}
