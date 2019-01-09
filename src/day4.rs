extern crate chrono;
extern crate itertools;
extern crate regex;
extern crate simple_error;

use std::cmp::Ord;
use std::collections::{BTreeMap, HashMap};
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;

use chrono::NaiveDateTime;
use chrono::Timelike;
use itertools::Itertools;
use regex::Regex;
use simple_error::SimpleError;

#[derive(Debug)]
enum LogEntry {
    BeginShift(usize),
    FallAsleep,
    WakeUp,
}

#[derive(Debug)]
struct Sleep {
    guard_id: usize,
    start: usize,
    end: usize,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        // XXX: is this really what you have to do to bubble up a simple
        // string error message?
        return Err(Box::new(SimpleError::new(format!(
            "expected exactly one argument, but got {}", args.len() - 1))));
    }

    // Parse the input into a sorted list of entries.
    let raw_log = {
        let line_re = Regex::new(r"^\[(.*)\] (.*)$")?;
        let shift_start_re = Regex::new(r"^Guard #(\d+) begins shift$")?;

        let input = BufReader::new(File::open(&args[1])?);
        let mut log = BTreeMap::new();
        for line in input.lines() {
            let line = line?; // XXX: ew.
            let captures = line_re.captures(&line)
                .ok_or("failed to parse line")?;

            let timestamp_str = captures.get(1).unwrap().as_str();
            let timestamp = NaiveDateTime::parse_from_str(
                timestamp_str, "%Y-%m-%d %H:%M")?;

            let message_str = captures.get(2).unwrap().as_str();
            let message = match message_str {
                "falls asleep" => LogEntry::FallAsleep,
                "wakes up" => LogEntry::WakeUp,
                _ => {
                    let captures = shift_start_re.captures(&message_str)
                        .ok_or("failed to parse message")?;
                    let id = captures.get(1).unwrap().as_str().parse::<usize>()?;
                    LogEntry::BeginShift(id)
                }
            };

            log.insert(timestamp, message);
        }
        log
    };

    // Parse the raw log into a list of sleeps.
    let log = {
        let mut log = Vec::new();
        let mut last_id = 0;
        let mut last_start = 0;
        for (timestamp, message) in raw_log.iter() {
            match message {
                LogEntry::BeginShift(id) => {
                    last_id = *id;
                },
                LogEntry::FallAsleep => {
                    last_start = timestamp.minute() as usize;
                },
                LogEntry::WakeUp => {
                    let s = Sleep{
                        guard_id: last_id,
                        start: last_start,
                        end: timestamp.minute() as usize,
                    };
                    log.push(s);
                }
            }
        }
        log
    };

    // Count how often each guard is asleep during each minute.
    let sleepy_minutes = {
        let mut tracker = HashMap::new();
        for s in log.iter() {
            let e = tracker.entry(s.guard_id).or_insert([0; 60]);
            for m in s.start..s.end {
                e[m as usize] += 1;
            }
        }
        tracker
    };

    // Part 1.
    {
        // Find the ID of the guard that sleeps the most.
        // XXX: This was arguably cleaner without iterators.
        let id = log.iter()
            .sorted_by(|s1, s2| Ord::cmp(&s1.guard_id, &s2.guard_id))
            .group_by(|s| s.guard_id)
            .into_iter()
            .map(|(g, ss)| (g, ss.map(|s| s.end - s.start).sum::<usize>()))
            .max_by_key(|&(_, d)| d)
            .map(|(g, _)| g)
            .ok_or("finding sleepiest guard failed")?;

        let m = sleepy_minutes[&id].iter()
            .enumerate()
            .max_by_key(|&(_, count)| count)
            .map(|(m, _)| m)
            .ok_or("finding sleepiest minute failed")?;

        println!("guard #{} slept the longest, and slept most during minute {}",
            id, m);
        println!("part 1 answer: {}", id * m);
    }

    // Part 2.
    {
        // XXX: This doesn't feel Rusty, but the iterator approach was harder to
        // read, in my opinion. (I made the opposite decision in the part 1 code
        // above.)
        let mut max_sleeps = 0;
        let mut max_id = 0;
        let mut max_minute = 0;
        for (guard, tracker) in sleepy_minutes.iter() {
            for (minute, sleeps) in tracker.iter().enumerate() {
                if *sleeps > max_sleeps {
                    max_sleeps = *sleeps;
                    max_id = *guard;
                    max_minute = minute;
                }
            }
        }
        println!("the absolute sleepiest minute was {} when guard #{} was on duty",
            max_minute, max_id);
        println!("part 2 answer: {}", max_id * max_minute)
    }


    Ok(())
}