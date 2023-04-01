use blake2::{digest::generic_array::sequence::Lengthen, Blake2b, Digest};
use clap::{command, value_parser, Arg, ArgAction};
use num_bigint::{BigUint, RandBigInt, ToBigUint};
use num_format::{SystemLocale, ToFormattedString};
use rand::rngs::ThreadRng;
use std::collections::HashSet;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::mpsc::{self, Sender};
use std::thread;
use std::time::Instant;
use std::{env, fs, io};

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

fn mk_nonce(nonce_digits: &String) -> String {
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
    nonce_digits: String,
    attempts: u64,
    seconds_since_last: u64,
    vanity_rate: (f32, &'static str),
    count: u64,
    thread_id: u16,
}

fn print(v: Vanity) {
    let Vanity {
        hash,
        nonce,
        nonce_digits: _,
        attempts,
        seconds_since_last,
        vanity_rate: (rate, rate_unit),
        count,
        thread_id,
    } = v;
    let locale = SystemLocale::default().unwrap();
    let fmt_attempts = attempts.to_formatted_string(&locale);
    println!("  ┌────────────────────────────────────────────");
    print!("{thread_id: >2}│ {nonce}");
    println!("  │ └> {hash}");
    println!("  │     found in: {seconds_since_last}s ({fmt_attempts} attempts)");
    println!("  │ found so far: {count} ({rate:.2}/{rate_unit})");
}

fn search_forever(
    thread_id: u16,
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
        let nonce_digits = mk_nonce_digits(&mut rng);
        let nonce = mk_nonce(&nonce_digits);
        let hash = get_hash(&nonce, hasher.clone());
        count += 1;
        if vanity_set.contains(&hash[..vanity_lenth].to_string()) {
            vanity_count += 1;
            let elapsed = start_time.elapsed().as_secs();
            let elapsed_total = t0.elapsed().as_secs();
            let total_rate = calc_rate(vanity_count, elapsed_total);
            let result = Vanity {
                hash,
                nonce,
                nonce_digits,
                attempts: count,
                seconds_since_last: elapsed,
                vanity_rate: total_rate,
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

fn calc_rate(count: u64, seconds: u64) -> (f32, &'static str) {
    let total_rate = if seconds > 0 {
        60. * count as f32 / seconds as f32
    } else {
        0.
    };
    if total_rate >= 1. {
        (total_rate, "minute")
    } else {
        (60. * total_rate, "hour")
    }
}

fn main() {
    let matches = command!() // requires `cargo` feature
        .arg(Arg::new("proto_file")
             .required(true)
             .help("Path to Tezos protocol source file.")
             .value_parser(value_parser!(PathBuf)))
        .arg(Arg::new("vanity_string")
             .required(true)
             .help("Look for protocol hashes starting with this string, e.g. PtMumbai"))
        .arg(
            Arg::new("ignore_case")
                .short('i')
                .long("ignore-case")
                .help("perform case insensitive matching")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("thread_count")
                .short('j')
                .long("thread-count")
                .help("number of threads to use (default: determine automatically based on the number of available cores/CPUs)")
                .value_parser(value_parser!(u16).range(1..)),
        )
        .arg(
            Arg::new("output_format")
                .short('f')
                .long("output-format")
                .help("Output format")
                .default_value("human")
                .value_parser(["human", "csv"])
        )
        .get_matches();

    let file_path = matches.get_one::<PathBuf>("proto_file").unwrap();

    let vanity = matches.get_one::<String>("vanity_string").unwrap();

    let ignore_case = matches.get_flag("ignore_case");

    let output_format = matches.get_one::<String>("output_format").unwrap().as_str();

    let mut vanity_set = HashSet::<String>::new();

    if ignore_case {
        for permutation in capitalization_permutations(&vanity[2..]) {
            let mut vanity_item = String::new();
            vanity_item.push_str(&vanity[..2]);
            vanity_item.push_str(&permutation[..]);
            vanity_set.insert(vanity_item);
        }
    } else {
        vanity_set.insert(vanity.to_string());
    }

    let thread_count = if let Some(thread_count) = matches.get_one::<u16>("thread_count") {
        *thread_count
    } else {
        let available_threads =
            thread::available_parallelism().unwrap_or(NonZeroUsize::try_from(1).unwrap());
        let available_threads: usize = available_threads.into();
        u16::try_from(available_threads).unwrap()
    };

    let file_path_str = file_path.to_str().unwrap();
    println!("Looking for vanity hash {vanity} for {file_path_str} using {thread_count} threads and vanity set {:?}", vanity_set);

    let data_as_bytes = fs::read(file_path).expect("Unable to read file");

    let vanity_comment_start = "(* Vanity nonce:";
    //protocol is a bunch of concatenated source files - mostly text, but with some bytes
    //in the mix that makes it invalid UTF8. We want to be able to use rfind though, hence
    //converting it to text in an unsafe way
    let maybe_vanity_start =
        unsafe { String::from_utf8_unchecked(data_as_bytes.clone()).rfind(vanity_comment_start) };

    let mut full_proto_hasher = Blake2b256::new();
    full_proto_hasher.update(&data_as_bytes[4..]);
    let raw_hash = full_proto_hasher.finalize().prepend(0xaa).prepend(0x02);
    let current_hash = bs58::encode(raw_hash).with_check().into_string();
    println!("Current hash: {current_hash}");

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

            if output_format == "csv" {
                let mut wtr = csv::Writer::from_writer(io::stdout());
                wtr.write_record([
                    "hash",
                    "nonce",
                    "thread",
                    "thread_attempts",
                    "thread_found_in_sec",
                    "thread_rate",
                    "thread_rate_unit",
                    "found_in_sec",
                    "rate",
                    "rate_unit",
                ])
                .unwrap();
                wtr.flush().unwrap();
            }

            loop {
                let start_time = Instant::now();
                let received = rx.recv().unwrap();
                vanity_count += 1;
                let elapsed = start_time.elapsed().as_secs();
                let elapsed_total = t0.elapsed().as_secs();
                let (total_rate, rate_unit) = calc_rate(vanity_count, elapsed_total);
                match output_format {
                    "csv" => {
                        let mut wtr = csv::Writer::from_writer(io::stdout());
                        wtr.write_record(&[
                            received.hash,
                            received.nonce_digits,
                            received.thread_id.to_string(),
                            received.attempts.to_string(),
                            received.seconds_since_last.to_string(),
                            received.vanity_rate.0.to_string(),
                            received.vanity_rate.1.to_string(),
                            elapsed.to_string(),
                            total_rate.to_string(),
                            rate_unit.to_string(),
                        ])
                        .unwrap();
                        wtr.flush().unwrap();
                    }
                    _ => {
                        print(received);
                        println!("  │      elapsed: {elapsed}s/{elapsed_total}s");
                        println!("  │  total found: {vanity_count} ({total_rate:.2}/{rate_unit})");
                        println!("  └────────────────────────────────────────────");
                    }
                }
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
