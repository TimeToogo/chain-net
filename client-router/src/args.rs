use clap::Clap;

#[derive(Clap, Clone)]
#[clap(version = "1.0", author = "Elliot Levin <elliotlevin@hotmail.com>")]
pub struct Args {
    pub interface: String,

    #[clap(long)]
    pub promisc: bool,

    #[clap(short, long, parse(from_occurrences))]
    pub dump: u16
}