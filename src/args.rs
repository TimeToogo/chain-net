use clap::Clap;

#[derive(Clap, Clone)]
#[clap(version = "1.0", author = "Elliot Levin <elliotlevin@hotmail.com>")]
pub struct Args {
    pub interface: String,

    pub port: u16
}