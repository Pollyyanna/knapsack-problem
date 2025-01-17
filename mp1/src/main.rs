use std::{ fmt, ops::RangeInclusive, time::Instant};
use rand::Rng;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

use tabled::{Table, Tabled};

#[derive(Copy, Clone)]
struct BitString {
    data: u64
}

impl BitString {
    fn new(data: u64) -> Self {
        Self { data  }
    }

    #[inline(always)]
    fn is_bit_set(&self, index: usize) -> bool {
        (self.data & (1u64 << index)) != 0
    }

    fn least_significant_bit(&self) -> usize {
        self.data.trailing_zeros() as usize
    }

    #[inline(always)]
    fn flip_bit(&mut self, index: usize) {
        self.data ^= 1 << index;
    }
}

impl fmt::Display for BitString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:08b}", self.data)
    }
}

struct Knapsack {
    weights: Vec<u16>,
    values: Vec<u16>,
    total_items: usize,
}

impl Knapsack {
    fn new(total_items: usize) -> Self {
        Self {
            weights: vec![0; total_items],
            values: vec![0; total_items],
            total_items,
        }
    }


    fn initialize_values(&mut self, weight_range: RangeInclusive<u16>, value_range: RangeInclusive<u16>) {
        for i in 0..self.total_items {
            let weight = rand::thread_rng().gen_range(weight_range.clone());
            let value = rand::thread_rng().gen_range(value_range.clone());
            self.weights[i] = weight;
            self.values[i] = value;
        }
    }

    fn print_weights_and_values(&self) {
        #[derive(Tabled)]
        struct Item {
            index: usize,
            weight: u16,
            value: u16,
        }

        let items: Vec<Item> = (0..self.total_items)
                .map(|i| Item {
                    index: i,
                    weight: self.weights[i],
                    value: self.values[i],
                })
                .collect();
        let table = Table::new(&items).to_string();
        println!("{}", table);
    }

    fn print_best_subset(&self, subset: BitString) {
        #[derive(Tabled)]
        struct Item {
            index: usize,
            weight: u16,
            value: u16,
        }


        let mut items: Vec<Item> = Vec::with_capacity(self.total_items);
        for i in 0..self.total_items {
            if subset.is_bit_set(i) {
                items.push(Item {
                    index: i,
                    weight: self.weights[i],
                    value: self.values[i]
                })
            }
        }

        let table = Table::new(&items).to_string();
        println!("{}", table);
    }

    fn solve(&self, knapsack_capacity: u64, multiprogress: MultiProgress, update_freq: u64) -> (BitString, u64) {

        // we ignore the 0 bit string since it doesn't have any value
        let mut bit_str = BitString::new(0);

        let mut max_value = 0;
        let mut best_subset = BitString::new(0);

        let mut current_weight = 0;
        let mut current_value = 0;

        let n = 1 << self.total_items;

        let pb = multiprogress.add(ProgressBar::new(n as u64));
        pb.set_style(
            ProgressStyle::with_template(
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] ({pos}/{len}, ETA {eta})",
            )
            .unwrap(),
        );

        let update_freq = n / update_freq;
        let mut next_update = update_freq; 

        for i in 1..n {
            let lsb = BitString::new(i).least_significant_bit();
            bit_str.flip_bit(lsb);

            // Only update progress bar at fixed intervals
            if i == next_update {
                pb.set_position(i);
                next_update += update_freq;
            }

            if bit_str.is_bit_set(lsb) {
                current_weight += self.weights[lsb] as u64;
                current_value += self.values[lsb] as u64;
            } else {
                current_weight -= self.weights[lsb] as u64;
                current_value -= self.values[lsb] as u64;
            }

            if current_weight > knapsack_capacity {
                continue
            }

            if max_value < current_value {
                max_value = current_value;
                best_subset = bit_str;
            }
        }

        pb.finish();

        return (best_subset, max_value)
    }
}

use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Number of total items must be in the range [1, 50]
    #[arg(long, value_parser = clap::value_parser!(u8).range(1..=50))]
    size: u8,

    /// Number of times to run the experiment
    #[arg(long, default_value_t = 3)]
    trials: usize,

    /// Number of times to update the progress bar
    #[arg(long, default_value_t = 1000)]
    update_freq: u64,

    /// Weight Minimum
    #[arg(long, default_value_t = 50)]
    weight_min: u16,

    /// Weight Maximum
    #[arg(long, default_value_t = 100)]
    weight_max: u16,

    /// Value Minimum
    #[arg(long, default_value_t = 100)]
    value_min: u16,

    /// Value Maximum
    #[arg(long, default_value_t = 500)]
    value_max: u16,

    /// Knapsack Capacity
    #[arg(long, default_value_t = 1000)]
    knapsack_capacity: u64,
}

use rayon::prelude::*;
use std::sync::{Arc, Mutex};

fn main() {
    let args = Args::parse();

    let trial_results = Arc::new(Mutex::new(vec![]));
    let multiprogress = MultiProgress::new();
    let print_lock = Arc::new(Mutex::new(()));


    (0..args.trials).into_par_iter().for_each(|i| {
        let mut knapsack = Knapsack::new(args.size as usize);

        let trial_results = trial_results.clone();
        let print_lock = print_lock.clone();

        knapsack.initialize_values(args.weight_min..=args.weight_max, args.value_min..=args.value_max);

        let now = Instant::now();
        let (subset, value) = knapsack.solve(args.knapsack_capacity, multiprogress.clone(), args.update_freq);
        let elapsed = now.elapsed().as_secs_f64();

        trial_results.lock().unwrap().push(elapsed);

        // Lock the printing section to ensure no overlap
        let _print_guard = print_lock.lock().unwrap();
        println!("-------------------------------- TRIAL {i} --------------------------------");
        knapsack.print_weights_and_values();
        println!("Done! Took {elapsed} seconds");
        println!("Best subset with value: {value} is");
        knapsack.print_best_subset(subset);
        println!("---------------------------------------------------------------------------");
        println!();
    });

    let trial_results = trial_results.lock().unwrap();
    println!("Took on average {}", trial_results.iter().sum::<f64>() / args.trials as f64);
}
