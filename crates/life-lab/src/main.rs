//! life-lab: analysis tools. Reads traces, checkpoints and the truth log; never feeds anything back
//! into a running life.
//!
//!   life-lab audit --run DIR
//!   life-lab summary --run DIR [--window 100000]
//!   life-lab compare --runs DIR1,DIR2,... [--actions-only]
//!   life-lab changes --run DIR
//!   life-lab gradcheck [--n 8 --ticks 80 --delta 0.01 --seed S --stream-seed S] [--ckpt FILE] [--out FILE]
//!   life-lab probe-history --run DIR [--a 200000 --b 1000000]
//!   life-lab usefulness --runs DIR1,DIR2,... [--at 200000]
//!   life-lab probe-order --runs DIR1,DIR2,... [--at 1000000]
//!   life-lab readapt --run DIR --at T_C [--fresh DIR --frozen DIR] [--out FILE]
//!   life-lab evaluate --root SEED_DIR --seed S
//!   life-lab gate --root RESULTS_DIR [--seeds 101-110]
//!   life-lab inspect --ckpt FILE [--run DIR] [--all]        (development diagnostic)
//!   life-lab devsweep [--seed S]                            (development diagnostic)

mod common;
mod gate;
mod gradcheck;
mod inspect;
mod probes;
mod traces;

fn main() {
    let (cmd, m) = common::parse_args();
    match cmd.as_str() {
        "audit" => traces::audit(&m),
        "summary" => traces::summary(&m),
        "compare" => traces::compare(&m),
        "changes" => traces::changes(&m),
        "gradcheck" => gradcheck::run(&m),
        "probe-history" => probes::probe_history_cmd(&m),
        "usefulness" => probes::usefulness_cmd(&m),
        "probe-order" => probes::probe_order_cmd(&m),
        "readapt" => probes::readapt_cmd(&m),
        "evaluate" => probes::evaluate_cmd(&m),
        "gate" => gate::run(&m),
        "inspect" => inspect::inspect(&m),
        "devsweep" => inspect::devsweep(&m),
        _ => {
            eprintln!("usage: life-lab audit|summary|compare|changes|gradcheck|probe-history|usefulness|probe-order|readapt|evaluate|gate ...");
            std::process::exit(2);
        }
    }
}
