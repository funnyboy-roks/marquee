use clap::Parser;
use serde::{Deserialize, Serialize};
use std::{
    io::{self, Write},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

/// Read stdin and output it in a marquee style
///
/// Once a line is read into stdin, it will stop the previous marquee and start the new one from
/// the beginning.
///
/// If an empty string is passed, then nothing is returned and it will keep waiting for more input.
///
/// This is intended for use with user-facing output, however, if one wants it to be used in a
/// pipeline of some sort, I'd recommend using `marquee -ld0`
///
/// See https://crates.io/crates/marquee for usage examples.
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Cli {
    /// Milliseconds to delay between every print
    #[arg(short, long, value_name = "ms", default_value_t = 1000)]
    delay: u64,

    /// The maximum width of each output line.
    ///
    /// If the length of the input < width, then it will just print the input.
    ///
    /// Note: This *only* impacts the moving content, the prefix/suffix is not included
    #[arg(short, long, value_name = "chars", default_value_t = 20)]
    width: usize,

    /// Prevent the marquee from looping
    ///
    /// This will only use the first line of the provided input.
    #[arg(short, long = "no-loop", action = clap::ArgAction::SetFalse)]
    _loop: bool,

    /// Prefix to print before every output line
    #[arg(short, long, value_name = "prefix")]
    prefix: Option<String>,

    /// Suffix to print after every output line
    #[arg(short = 'f', long, value_name = "suffix")]
    suffix: Option<String>,

    /// Separator to use between entries when looping.
    ///
    /// Note: This is not used when `no-loop` is set
    #[arg(short, long, value_name = "sep", default_value_t = String::from("    "))]
    separator: String,

    /// Reverse the output (starts at the far right and move left)
    #[arg(short, long)]
    reverse: bool,

    /// Print the output on the same line, using the `\r` escape code.
    #[arg(short = 'L', long)]
    same_line: bool,

    /// If the input will be passed in as JSON
    #[arg(short, long)]
    json: bool,
}

/// A function which returns true (for serde default)
fn default_true() -> bool {
    true
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct JsonInput {
    /// The prefix to put before the content
    #[serde(default)]
    prefix: String,

    /// The actual content to rotate
    content: String,

    /// The suffix to put after the content
    #[serde(default)]
    suffix: String,

    /// If the line should rotate
    #[serde(default = "default_true")]
    rotate: bool,
}

fn utf_substring(string: &String, start: usize, count: usize) -> String {
    let mut out_chars = string.chars();
    if start > 0 {
        out_chars.nth(start - 1); // Remove up until i
    }
    return out_chars.take(count).collect(); // Take the rest (similar to out[i..i+len])
}

/// Start the timer thread that will run the clock for the outputs
fn start_timer(current_str: &Arc<Mutex<Option<String>>>, options: Cli) -> thread::JoinHandle<()> {
    let arc_str = Arc::clone(current_str);
    thread::spawn(move || {
        let wait_time = Duration::from_millis(options.delay);

        let mut i = 0;
        // The previous value that was shown, this is used for knowing when to reset `i`
        let mut prev = String::new();
        let mut prev_out = String::new();
        loop {
            let start = Instant::now();
            let str_value = arc_str.lock().unwrap();

            // If there is no input, don't print anything
            if str_value.is_none() || str_value.as_ref().unwrap().is_empty() {
                // Manually drop the lock on `arc_str` so that the stdin thread can put
                // something new into it.
                // (this is probably not the best way, but it works :shrug:)
                drop(str_value);

                // sleep so that it doesn't loop as fast as possible and devour the CPU (totally
                // not known from personal experience)
                if let Some(remaining) = wait_time.checked_sub(start.elapsed()) {
                    thread::sleep(remaining);
                }

                continue;
            }

            let mut out = str_value.as_ref().expect("error handled above").clone(); // Clone the string so that it can be used
            drop(str_value); // Drop `str_value` to remove the lock on `arc_str`.

            // If `--json`, then parse the json
            let json: Option<Result<JsonInput, _>> =
                options.json.then(|| serde_json::from_str(&out));

            if json.is_some() {
                if let Some(Err(err)) = &json {
                    eprintln!("Error parsing JSON: {:?}", err);
                    *arc_str.lock().unwrap() = None; // Reset the string because
                                                     // there's no reason to keep trying
                                                     // to parse the json
                    if let Some(remaining) = wait_time.checked_sub(start.elapsed()) {
                        thread::sleep(remaining);
                    }
                    continue;
                }
            }

            let json = json.map(|c| c.expect("error handled above"));

            // If there is json, grab the string
            if let Some(JsonInput { content, .. }) = &json {
                out = content.clone();
            }

            // If the string has changed, then reset `i`
            if prev != out {
                i = if !options.reverse {
                    0
                } else {
                    out.len() * 2 - options.width
                };
            }
            prev = out.clone();

            let raw_len = out.len();
            if options.width < out.len() {
                // Put the separator at the beginning/end depending on whether --reverse is set
                let new = if options.reverse {
                    format!("{}{}", options.separator, out)
                } else {
                    format!("{}{}", out, options.separator)
                }
                .repeat(2); // Repeat twice so that we loop properly

                out = utf_substring(&new, i, options.width);

                // Only change `i` if this single string will be rotated, which is only true if
                // the input length > width and json.rotate is true
                if raw_len > options.width && (json.is_none() || json.clone().unwrap().rotate) {
                    if options.reverse {
                        if i == 0 {
                            // If the i is 0, set it to the end
                            i = new.len() - 1;
                        } else {
                            // Otherwise, decrement
                            i -= 1;
                        }
                    } else {
                        i += 1;
                        i %= raw_len + options.separator.len();
                    }
                }
            }

            // Add prefixes
            if let Some(ref prefix) = options.prefix {
                out = format!("{}{}", prefix, out);
            }
            if let Some(JsonInput { prefix, .. }) = &json {
                out = format!("{}{}", prefix, out);
            }

            // Add suffixes
            if let Some(JsonInput { suffix, .. }) = &json {
                out += suffix;
            }
            if let Some(ref suffix) = options.suffix {
                out += suffix;
            }

            // Break after printing everything when `--no-loop` is passed
            if !options._loop && i + options.width == raw_len + 2 {
                break;
            }

            if options.same_line {
                print!("\r{}", out);
                if prev_out.len() > out.len() {
                    // Clear the rest of the line
                    print!("{}", " ".repeat(prev_out.len() - out.len()));
                }
                prev_out = out;
                io::stdout().flush().unwrap();
            } else {
                println!("{}", out);
            }

            // Sleep this thread for however much time is left until the delay is over
            if let Some(remaining) = wait_time.checked_sub(start.elapsed()) {
                thread::sleep(remaining);
            }
        }
    })
}

fn main() {
    let options = Cli::parse();
    let current_str = Arc::new(Mutex::new(Default::default()));

    let timer = start_timer(&current_str, options);

    // Thread that will listen to stdin and read each line, changing `current_str` to the latest line
    let input = thread::spawn(move || {
        let stdin = io::stdin();
        let lines = stdin.lines();
        for line in lines {
            let mut lock = current_str.lock().unwrap();
            *lock = Some(line.unwrap());
        }
    });

    input.join().expect("Failed while reading stdin");
    timer.join().expect("Failed while creating output");
}
