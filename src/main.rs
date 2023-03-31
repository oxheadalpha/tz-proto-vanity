use blake2::{digest::generic_array::sequence::Lengthen, Blake2b, Digest};
use num_bigint::{BigUint, RandBigInt, ToBigUint};
use rand::rngs::ThreadRng;
use std::collections::HashSet;
use std::num::NonZeroUsize;
use std::str::FromStr;
use std::sync::mpsc::{self, Sender};
use std::thread;
use std::time::Instant;
use std::{env, fs};

pub type Blake2b256 = Blake2b<blake2::digest::consts::U32>;

const BASE10: u64 = 10;
const NUM_NONCE_DIGITS: u8 = 16;
const MAX_NONCE: u64 = BASE10.pow(NUM_NONCE_DIGITS as u32);
const NUM_NONCE_DIGITS_USIZE: usize = NUM_NONCE_DIGITS as usize;

fn mk_nonce_digits(rng: &mut ThreadRng) -> String {
    let unsigned: BigUint =
        rng.gen_biguint_range(&0.to_biguint().unwrap(), &MAX_NONCE.to_biguint().unwrap());
    format!("{0:0NUM_NONCE_DIGITS_USIZE$}", unsigned)
}

fn mk_nonce(nonce_digits: String) -> String {
    format!("(* Vanity nonce: {nonce_digits} *)\n")
}

fn get_hash(vanity_nonce: &str, mut proto_hash: Blake2b256) -> String {
    proto_hash.update(vanity_nonce.as_bytes());
    let raw_hash = proto_hash.finalize().prepend(0xaa).prepend(0x02);
    bs58::encode(raw_hash).with_check().into_string()
}

struct Vanity {
    hash: String,
    nonce: String,
    attempts: u64,
    seconds_since_last: u64,
    vanity_per_minute: u64,
    count: u64,
    thread_id: usize,
}

fn print(v: Vanity) {
    let Vanity {
        hash,
        nonce,
        attempts,
        seconds_since_last,
        vanity_per_minute,
        count,
        thread_id,
    } = v;
    println!("  ┌────────────────────────────────────────────");
    print!("{thread_id: >2}│ {nonce}");
    println!("  │ └> {hash}");
    println!("  │ found in: {seconds_since_last}s in {attempts} attempts");
    println!("  │ found so far: {count} ({vanity_per_minute}/minute)");
}

fn search_forever(
    thread_id: usize,
    tx: Sender<Vanity>,
    vanity_set: HashSet<String>,
    hasher: Blake2b256,
) {
    let vanity_lenth = vanity_set.iter().next().unwrap().len();
    let mut count = 0u64;
    let mut vanity_count = 0u64;
    let t0 = Instant::now();
    let mut start_time = Instant::now();
    let mut rng = rand::thread_rng();
    loop {
        let nonce = mk_nonce(mk_nonce_digits(&mut rng));
        let hash = get_hash(&nonce, hasher.clone());
        count += 1;
        if vanity_set.contains(&hash[..vanity_lenth].to_string()) {
            vanity_count += 1;
            let elapsed = start_time.elapsed().as_secs();
            let elapsed_total = t0.elapsed().as_secs();
            let total_rate = if elapsed_total > 0 {
                60 * vanity_count / elapsed_total
            } else {
                0
            };

            let result = Vanity {
                hash,
                nonce,
                attempts: count,
                seconds_since_last: elapsed,
                vanity_per_minute: total_rate,
                count: vanity_count,
                thread_id,
            };

            tx.send(result).unwrap();
            start_time = Instant::now();
            count = 0
        }
    }
}

fn capitalization_permutations(s: &str) -> Vec<String> {
    if s.is_empty() {
        vec![String::new()]
    } else {
        let first = &s[0..1];
        let rest = &s[1..];
        let mut result = vec![];
        for p in capitalization_permutations(rest) {
            let mut upper = String::new();
            upper.push_str(&first.to_uppercase());
            upper.push_str(&p);
            result.push(upper);
            if first.to_uppercase() != first.to_lowercase() {
                let mut lower = String::new();
                lower.push_str(&first.to_lowercase());
                lower.push_str(&p);
                result.push(lower);
            }
        }
        result
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let file_path = &args[1];
    let exact_or_caseignore = &args[2];
    let vanity = &args[3];

    let mut vanity_set = HashSet::<String>::new();

    if exact_or_caseignore == "exact" {
        vanity_set.insert(vanity.to_string());
    } else if exact_or_caseignore == "caseignore" {
        for permutation in capitalization_permutations(&vanity[2..]) {
            let mut vanity_item = String::new();
            vanity_item.push_str(&vanity[..2]);
            vanity_item.push_str(&permutation[..]);
            vanity_set.insert(vanity_item);
        }
    } else {
        panic!("arg before vanity string must be 'exact' or 'caseignore'");
    }

    let thread_count = if args.len() == 5 {
        str::parse(&args[4]).unwrap()
    } else {
        let available_threads =
            thread::available_parallelism().unwrap_or(NonZeroUsize::try_from(1).unwrap());
        let available_threads: usize = available_threads.into();
        available_threads
    };

    println!("Looking for vanity hash {vanity} for {file_path} using {thread_count} threads and vanity set {:?}", vanity_set);

    let data_as_bytes = fs::read(file_path).expect("Unable to read file");
    let data = String::from_utf8(data_as_bytes.clone()).expect("Unable to decode as UTF-8 text");
    let vanity_comment_start = "(* Vanity nonce:";
    let maybe_vanity_start = data.rfind(vanity_comment_start);

    match maybe_vanity_start {
        Some(vanity_start) => {
            //First 4 bytes are truncated (not used in proto hashing)
            let up_to_vanity = &data_as_bytes[4..vanity_start];
            let mut hasher = Blake2b256::new();
            hasher.update(up_to_vanity);

            let (tx, rx) = mpsc::channel::<Vanity>();

            for i in 1..=thread_count {
                let vanity_set_copy = vanity_set.clone();
                let hasher_copy = hasher.clone();
                let sender_copy = tx.clone();
                thread::spawn(move || search_forever(i, sender_copy, vanity_set_copy, hasher_copy));
            }

            let t0 = Instant::now();
            let mut vanity_count = 0u64;
            loop {
                let start_time = Instant::now();
                let received = rx.recv().unwrap();
                vanity_count += 1;
                print(received);
                let elapsed = start_time.elapsed().as_secs();
                let elapsed_total = t0.elapsed().as_secs();
                let total_rate = if elapsed_total > 0 {
                    60 * vanity_count / elapsed_total
                } else {
                    0
                };
                println!("  │ elapsed: {elapsed}s/{elapsed_total}s");
                println!("  │ total found: {vanity_count} ({total_rate}/minute)");
                println!("  └────────────────────────────────────────────");
            }
        }
        None => println!("Vanity comment line start '{vanity_comment_start}' is not found "),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capitalization_permutations() {
        let mut actual = capitalization_permutations("abc");
        let mut expected = vec!["ABC", "aBC", "AbC", "abC", "ABc", "aBc", "Abc", "abc"];
        actual.sort();
        expected.sort();
        assert_eq!(actual, expected);

        let actual = capitalization_permutations("");
        let expected = vec![""];
        assert_eq!(actual, expected);

        let mut actual = capitalization_permutations("X*Y");
        let mut expected = vec!["X*Y", "x*Y", "X*y", "x*y"];
        actual.sort();
        expected.sort();
        assert_eq!(actual, expected);
    }
}
