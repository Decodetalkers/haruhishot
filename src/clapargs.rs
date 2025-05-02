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
        long_flag = "list-outputs",
        short_flag = 'L',
        about = "list all outputs"
    )]
    ListOutputs,
    #[command(long_flag = "output", short_flag = 'O', about = "choose output")]
    Output {
        #[arg(required = false)]
        output: Option<String>,
        #[arg(value_name = "stdout", long)]
        stdout: bool,
        #[arg(value_name = "pointer", long, default_value = "false")]
        cursor: bool,
    },
    #[command(long_flag = "slurp", short_flag = 'S', about = "area select")]
    Slurp {
        #[arg(value_name = "stdout", long)]
        stdout: bool,
        #[arg(value_name = "pointer", long, default_value = "false")]
        cursor: bool,
    },
}
