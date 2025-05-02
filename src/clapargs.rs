use clap::{Parser, arg};

#[derive(Debug, Parser, PartialEq, Eq)]
#[command(
    name = "haruhishot",
    about="One day Haruhi Suzumiya made a wlr screenshot tool",
    long_about = None,
    version,
)]
pub enum HaruhiCli {
    #[command(
        long_flag = "list_outptus",
        short_flag = 'L',
        about = "list all outputs"
    )]
    ListOutputs,
    #[command(
        long_flag = "list_outptus",
        short_flag = 'L',
        about = "list all outputs"
    )]
    #[command(long_flag = "output", short_flag = 'O', about = "choose output")]
    Output {
        #[arg(required = false)]
        output: Option<String>,
        #[arg(value_name = "stdout", long)]
        stdout: bool,
    },
}
